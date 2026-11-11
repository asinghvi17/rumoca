//! Scalar equation rows for the `base-modelica-scalar` target.
//!
//! Two template functions turn Flat equation rows into one scalar equation per
//! array element, the form a consumer without array support reads:
//!
//! * `scalarize_equation(eq, variables)`: an equation over whole arrays
//!   (`der(x) = -(a .* x)`) becomes one equation per element when every
//!   operation in it is elementwise; anything else (a matrix product, a
//!   reduction over an array argument, a slice, a call taking an array) is
//!   refused rather than guessed.
//! * `unroll_family(loop, variables)`: an unmaterialized `for` family (whose
//!   Flat rows are placeholders) becomes its template body at every domain
//!   point, binders substituted and constant subscripts folded to indices.
//!
//! Both work on the serialized Flat expression form the renderer reads, and
//! leave the result to `render_flat_equation`.

use crate::errors::render_err;
use minijinja::Value;
use serde_json::{Value as Json, json};

type Expansion<T> = Result<T, minijinja::Error>;

/// `scalarize_equation(eq, variables)`: the scalar rows of one equation.
pub(super) fn scalarize_equation_function(eq: Value, variables: Value) -> Expansion<Value> {
    let eq = to_json(&eq)?;
    let rows = scalar_rows(&eq, &variables)?;
    Ok(Value::from_serialize(rows))
}

/// `unroll_family(loop, variables)`: the scalar rows of a `for` family row.
pub(super) fn unroll_family_function(family: Value, variables: Value) -> Expansion<Value> {
    let family = to_json(&family)?;
    let binders = family["binders"]
        .as_array()
        .ok_or_else(|| render_err("unroll_family: loop row has no binders"))?;
    let mut names = Vec::with_capacity(binders.len());
    let mut ranges = Vec::with_capacity(binders.len());
    for binder in binders {
        names.push(str_field(binder, "name")?.to_string());
        ranges.push(binder_values(binder)?);
    }
    let points = cartesian(&ranges)
        .into_iter()
        .map(|values| names.iter().cloned().zip(values).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let body = family["equations"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let mut rows = Vec::new();
    for point in &points {
        for eq in body {
            let mut eq = eq.clone();
            substitute_binders(&mut eq["residual"], point);
            eq["scalar_count"] = json!(1);
            rows.extend(scalar_rows(&eq, &variables)?);
        }
    }
    Ok(Value::from_serialize(rows))
}

/// The values a `lower:step:upper` binder takes, in order.
fn binder_values(binder: &Json) -> Expansion<Vec<i64>> {
    let bound = |key: &str| {
        binder[key]
            .as_i64()
            .ok_or_else(|| render_err(format!("unroll_family: binder has no integer `{key}`")))
    };
    let (lower, upper, step) = (bound("lower")?, bound("upper")?, bound("step")?);
    if step == 0 {
        return Err(render_err("unroll_family: binder has step 0"));
    }
    Ok((0..)
        .map(|k| lower + k * step)
        .take_while(|v| if step > 0 { *v <= upper } else { *v >= upper })
        .collect())
}

/// Every combination of one value per axis, the last axis varying fastest.
fn cartesian<T: Copy>(axes: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut rows = vec![Vec::new()];
    for axis in axes {
        rows = rows
            .iter()
            .flat_map(|row| axis.iter().map(move |value| extended(row, *value)))
            .collect();
    }
    rows
}

fn extended<T: Copy>(row: &[T], value: T) -> Vec<T> {
    let mut next = row.to_vec();
    next.push(value);
    next
}

fn to_json(value: &Value) -> Expansion<Json> {
    serde_json::to_value(value)
        .map_err(|err| render_err(format!("base-modelica-scalar: unserializable row: {err}")))
}

/// One equation per element of the equation's array shape (itself if scalar).
fn scalar_rows(eq: &Json, variables: &Value) -> Expansion<Vec<Json>> {
    let mut eq = eq.clone();
    fold_subscripts(&mut eq["residual"]);
    let residual = &eq["residual"];
    let Some(shape) = array_shape(residual, variables)? else {
        return Ok(vec![eq]);
    };
    let mut rows = Vec::new();
    for index in element_indices(&shape) {
        let mut row = eq.clone();
        row["residual"] = element(residual, &index, variables)?;
        row["scalar_count"] = json!(1);
        rows.push(row);
    }
    Ok(rows)
}

fn element_indices(shape: &[usize]) -> Vec<Vec<usize>> {
    let axes = shape
        .iter()
        .map(|extent| (1..=*extent).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    cartesian(&axes)
}

/// Replace each loop binder reference with its integer value.
fn substitute_binders(expr: &mut Json, point: &[(String, i64)]) {
    if let Some(var_ref) = expr.get("VarRef")
        && var_ref["subscripts"].as_array().is_none_or(Vec::is_empty)
        && let Some((_, value)) = point
            .iter()
            .find(|(name, _)| Some(name.as_str()) == ref_name(var_ref))
    {
        *expr = json!({ "Literal": { "value": { "Integer": value } } });
        return;
    }
    match expr {
        Json::Object(map) => map
            .values_mut()
            .for_each(|child| substitute_binders(child, point)),
        Json::Array(items) => items
            .iter_mut()
            .for_each(|child| substitute_binders(child, point)),
        _ => {}
    }
}

/// Turn every constant expression subscript into an `Index`.
fn fold_subscripts(expr: &mut Json) {
    if let Some(subs) = expr
        .get_mut("VarRef")
        .and_then(|var_ref| var_ref.get_mut("subscripts"))
        .and_then(Json::as_array_mut)
    {
        for sub in subs.iter_mut() {
            if let Some(value) = sub
                .get("Expr")
                .map(|e| e.get("expr").unwrap_or(e))
                .and_then(eval_int)
            {
                *sub = json!({ "Index": value });
            }
        }
    }
    match expr {
        Json::Object(map) => map.values_mut().for_each(fold_subscripts),
        Json::Array(items) => items.iter_mut().for_each(fold_subscripts),
        _ => {}
    }
}

/// The value of an integer constant expression (literals and `+ - *`, and
/// exact `/`).
fn eval_int(expr: &Json) -> Option<i64> {
    if let Some(literal) = expr.get("Literal") {
        let value = literal.get("value").unwrap_or(literal);
        let int = value.get("Integer")?;
        return int.as_i64().or_else(|| int.get("value")?.as_i64());
    }
    if let Some(unary) = expr.get("Unary") {
        let rhs = eval_int(&unary["rhs"])?;
        return match unary["op"].as_str()? {
            "Minus" | "DotMinus" => Some(-rhs),
            "Plus" | "DotPlus" => Some(rhs),
            _ => None,
        };
    }
    let binary = expr.get("Binary")?;
    let (lhs, rhs) = (eval_int(&binary["lhs"])?, eval_int(&binary["rhs"])?);
    match binary["op"].as_str()? {
        "Add" | "AddElem" => lhs.checked_add(rhs),
        "Sub" | "SubElem" => lhs.checked_sub(rhs),
        "Mul" | "MulElem" => lhs.checked_mul(rhs),
        "Div" | "DivElem" if rhs != 0 && lhs % rhs == 0 => Some(lhs / rhs),
        _ => None,
    }
}

fn ref_name(var_ref: &Json) -> Option<&str> {
    let name = &var_ref["name"];
    name.as_str().or_else(|| name["name"].as_str())
}

/// The declared dimensions of variable `name` (empty for a scalar or an
/// undeclared name such as a function input).
fn declared_dims(variables: &Value, name: &str) -> Vec<usize> {
    variables
        .get_item(&Value::from(name))
        .ok()
        .and_then(|var| var.get_attr("dims").ok())
        .and_then(|dims| dims.try_iter().ok())
        .map_or_else(Vec::new, |dims| {
            dims.filter_map(|d| d.as_i64().and_then(|d| usize::try_from(d).ok()))
                .collect()
        })
}

/// Built-ins applied element by element to array arguments.
const ELEMENTWISE_BUILTINS: &[&str] = &[
    "Der",
    "Pre",
    "Abs",
    "Sign",
    "Sqrt",
    "Sin",
    "Cos",
    "Tan",
    "Asin",
    "Acos",
    "Atan",
    "Atan2",
    "Sinh",
    "Cosh",
    "Tanh",
    "Exp",
    "Log",
    "Log10",
    "Floor",
    "Ceil",
    "Integer",
    "Div",
    "Mod",
    "Rem",
    "NoEvent",
    "Smooth",
    "Homotopy",
    "Edge",
    "Change",
    "SemiLinear",
];

/// The array shape of `expr` if it is array-valued, `None` if scalar. A
/// form whose shape is not elementwise-determinable is refused.
fn array_shape(expr: &Json, variables: &Value) -> Expansion<Option<Vec<usize>>> {
    if let Some(var_ref) = expr.get("VarRef") {
        let name = ref_name(var_ref).ok_or_else(|| refusal(expr, "a reference without a name"))?;
        let dims = declared_dims(variables, name);
        let subs = var_ref["subscripts"].as_array().map_or(0, Vec::len);
        return match subs {
            0 if !dims.is_empty() => Ok(Some(dims)),
            n if n == dims.len() || dims.is_empty() => {
                if var_ref["subscripts"]
                    .as_array()
                    .is_some_and(|subs| subs.iter().any(|s| s.get("Index").is_none()))
                {
                    return Err(refusal(expr, "a non-constant or slice subscript"));
                }
                scalar_valued()
            }
            _ => Err(refusal(expr, "a partially subscripted array")),
        };
    }
    if expr.get("Literal").is_some() {
        return scalar_valued();
    }
    if let Some(array) = expr.get("Array") {
        let elements = array["elements"].as_array().map_or(&[][..], Vec::as_slice);
        let inner = match elements.first() {
            Some(first) => array_shape(first, variables)?.unwrap_or_else(Vec::new),
            None => Vec::new(),
        };
        let mut shape = vec![elements.len()];
        shape.extend(inner);
        return Ok(Some(shape));
    }
    let children: Vec<&Json> = if let Some(binary) = expr.get("Binary") {
        let lhs = array_shape(&binary["lhs"], variables)?;
        let rhs = array_shape(&binary["rhs"], variables)?;
        let op = str_field(binary, "op")?;
        if lhs.is_some() && rhs.is_some() && matches!(op, "Mul" | "Div" | "Exp") {
            return Err(refusal(expr, "a matrix operator"));
        }
        vec![&binary["lhs"], &binary["rhs"]]
    } else if let Some(unary) = expr.get("Unary") {
        vec![&unary["rhs"]]
    } else if let Some(builtin) = expr.get("BuiltinCall") {
        let args = builtin["args"]
            .as_array()
            .map_or_else(Vec::new, |a| a.iter().collect());
        let function = str_field(builtin, "function")?;
        if !ELEMENTWISE_BUILTINS.contains(&function) {
            return no_array_inside(expr, args, variables);
        }
        args
    } else if let Some(if_expr) = expr.get("If") {
        let mut children = vec![&if_expr["else_branch"]];
        for branch in if_expr["branches"].as_array().into_iter().flatten() {
            children.extend(branch.as_array().into_iter().flatten());
        }
        children
    } else if let Some(call) = expr.get("FunctionCall") {
        let args = call["args"]
            .as_array()
            .map_or_else(Vec::new, |a| a.iter().collect());
        return no_array_inside(expr, args, variables);
    } else {
        return no_array_inside(expr, Vec::new(), variables);
    };
    let mut shape = None;
    for child in children {
        if let Some(child_shape) = array_shape(child, variables)? {
            if shape
                .as_ref()
                .is_some_and(|s: &Vec<usize>| *s != child_shape)
            {
                return Err(refusal(expr, "operands of different shapes"));
            }
            shape = Some(child_shape);
        }
    }
    Ok(shape)
}

/// A call or other form is scalar only if none of its arguments is an array.
fn no_array_inside(
    expr: &Json,
    args: Vec<&Json>,
    variables: &Value,
) -> Expansion<Option<Vec<usize>>> {
    for arg in args {
        if array_shape(arg, variables)?.is_some() {
            return Err(refusal(expr, "a call with an array argument"));
        }
    }
    if ["Range", "ArrayComprehension", "Tuple"]
        .iter()
        .any(|kind| expr.get(kind).is_some())
    {
        return Err(refusal(expr, "an array-valued expression"));
    }
    scalar_valued()
}

/// The shape of a scalar-valued expression: no array shape.
fn scalar_valued() -> Expansion<Option<Vec<usize>>> {
    Ok(Option::None)
}

/// A required string field of a serialized Flat node.
fn str_field<'a>(node: &'a Json, key: &str) -> Expansion<&'a str> {
    node[key]
        .as_str()
        .ok_or_else(|| render_err(format!("base-modelica-scalar: node has no `{key}`: {node}")))
}

/// Element `index` of an elementwise array expression.
fn element(expr: &Json, index: &[usize], variables: &Value) -> Expansion<Json> {
    if array_shape(expr, variables)?.is_none() {
        return Ok(expr.clone());
    }
    if let Some(var_ref) = expr.get("VarRef") {
        let mut var_ref = var_ref.clone();
        var_ref["subscripts"] = Json::Array(index.iter().map(|i| json!({ "Index": i })).collect());
        return Ok(json!({ "VarRef": var_ref }));
    }
    if let Some(array) = expr.get("Array") {
        let (first, rest) = index
            .split_first()
            .ok_or_else(|| refusal(expr, "an array literal of the wrong rank"))?;
        let item = array["elements"]
            .get(first - 1)
            .ok_or_else(|| refusal(expr, "an array literal of the wrong size"))?;
        return element(item, rest, variables);
    }
    let mut out = expr.clone();
    if let Some(map) = out.as_object_mut()
        && let Some((_, node)) = map.iter_mut().next()
    {
        // Between scalars an elementwise operator is the plain one.
        if let Some(op) = node.get("op").and_then(Json::as_str)
            && let Some(scalar_op) = op.strip_suffix("Elem")
        {
            node["op"] = json!(scalar_op);
        }
        for key in ["lhs", "rhs", "else_branch"] {
            if let Some(child) = node.get(key).cloned() {
                node[key] = element(&child, index, variables)?;
            }
        }
        for key in ["args", "branches"] {
            if let Some(children) = node.get(key).and_then(Json::as_array).cloned() {
                node[key] = Json::Array(elements_of_children(&children, key, index, variables)?);
            }
        }
    }
    Ok(out)
}

/// Element `index` of each call argument, or of both halves of each `If`
/// branch (a `[condition, value]` pair).
fn elements_of_children(
    children: &[Json],
    key: &str,
    index: &[usize],
    variables: &Value,
) -> Expansion<Vec<Json>> {
    children
        .iter()
        .map(|child| match child.as_array() {
            Some(pair) if key == "branches" => pair
                .iter()
                .map(|part| element(part, index, variables))
                .collect::<Expansion<Vec<_>>>()
                .map(Json::Array),
            _ => element(child, index, variables),
        })
        .collect()
}

fn refusal(expr: &Json, reason: &str) -> minijinja::Error {
    let kind = expr
        .as_object()
        .and_then(|map| map.keys().next())
        .map_or("expression", String::as_str);
    render_err(format!(
        "cannot scalarize: the equation contains {reason} (a `{kind}` node); \
         use the `base-modelica` target"
    ))
}
