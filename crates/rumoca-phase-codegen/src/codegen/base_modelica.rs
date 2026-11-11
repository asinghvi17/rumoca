//! Base Modelica (MCP-0031) rendering helpers for the `base-modelica` target.
//!
//! Template functions: `quote_ident` (a Q-IDENT), `is_builtin_alias` (a
//! function every call of which renders as a built-in), `scalar_indices` (the
//! row-major element subscripts of an array shape) and `array_element` (one
//! element of an array-valued attribute), the last two used when the target
//! scalarizes arrays.
//!
//! Algorithm statements: function bodies and `algorithm` sections of the `base-modelica` target:
//! `:=` assignments, every block closed (`end if;`, `end for;`, `end while;`,
//! `end when;`), component references quoted through the same declared-name
//! set as expressions, and each statement terminated by `;`. Expressions are
//! Flat expressions, rendered by [`render_expression`].

use super::render_expr::{get_field, render_args, render_expression, render_serialized_name};
use super::{ExprConfig, RenderResult};
use crate::errors::render_err;
use minijinja::Value;

/// Render a statement list, one statement per line.
pub(crate) fn render_statements(stmts: &Value, cfg: &ExprConfig, indent: &str) -> RenderResult {
    let len = stmts
        .len()
        .ok_or_else(|| render_err("statement list is not a sequence"))?;
    let mut lines = Vec::with_capacity(len);
    for i in 0..len {
        let stmt = stmts
            .get_item(&Value::from(i))
            .map_err(|err| render_err(format!("statement {i} is inaccessible: {err}")))?;
        let line = render_statement(&stmt, cfg, indent)?;
        if !line.is_empty() {
            lines.push(line);
        }
    }
    Ok(lines.join("\n"))
}

/// Render one statement (MLS §11.2) in Base Modelica syntax.
pub(crate) fn render_statement(stmt: &Value, cfg: &ExprConfig, indent: &str) -> RenderResult {
    if let Some(unit) = stmt.as_str() {
        return match unit {
            "Empty" => Ok(String::new()),
            "Return" => Ok(format!("{indent}return;")),
            "Break" => Ok(format!("{indent}break;")),
            _ => Err(render_err(format!("unhandled statement variant: {unit}"))),
        };
    }
    if let Ok(spanned) = get_field(stmt, "Spanned") {
        return render_statement(&get_field(&spanned, "statement")?, cfg, indent);
    }
    if get_field(stmt, "Empty").is_ok() {
        return Ok(String::new());
    }
    if get_field(stmt, "Return").is_ok() {
        return Ok(format!("{indent}return;"));
    }
    if get_field(stmt, "Break").is_ok() {
        return Ok(format!("{indent}break;"));
    }
    if let Ok(assign) = get_field(stmt, "Assignment") {
        let target = render_component_ref(&get_field(&assign, "comp")?, cfg)?;
        let value = render_expression(&get_field(&assign, "value")?, cfg)?;
        return Ok(format!("{indent}{target} := {value};"));
    }
    if let Ok(for_stmt) = get_field(stmt, "For") {
        return render_for(&for_stmt, cfg, indent);
    }
    if let Ok(while_stmt) = get_field(stmt, "While") {
        let block = get_field(&while_stmt, "block")?;
        let cond = render_expression(&get_field(&block, "cond")?, cfg)?;
        let body = render_block(&get_field(&block, "stmts")?, cfg, indent)?;
        return Ok(format!(
            "{indent}while {cond} loop\n{body}{indent}end while;"
        ));
    }
    if let Ok(if_stmt) = get_field(stmt, "If") {
        return render_branches(&if_stmt, "cond_blocks", ["if", "elseif"], cfg, indent);
    }
    if let Ok(when_stmt) = get_field(stmt, "When") {
        return render_branches(&when_stmt, "blocks", ["when", "elsewhen"], cfg, indent);
    }
    if let Ok(call) = get_field(stmt, "FunctionCall") {
        return render_call(&call, cfg, indent);
    }
    if let Ok(reinit) = get_field(stmt, "Reinit") {
        let target = render_component_ref(&get_field(&reinit, "variable")?, cfg)?;
        let value = render_expression(&get_field(&reinit, "value")?, cfg)?;
        return Ok(format!("{indent}reinit({target}, {value});"));
    }
    if let Ok(assert) = get_field(stmt, "Assert") {
        let cond = render_expression(&get_field(&assert, "condition")?, cfg)?;
        let message = render_expression(&get_field(&assert, "message")?, cfg)?;
        let level = match get_field(&assert, "level") {
            Ok(level) if !level.is_none() => format!(", {}", render_expression(&level, cfg)?),
            _ => String::new(),
        };
        return Ok(format!("{indent}assert({cond}, {message}{level});"));
    }
    Err(render_err(format!(
        "base-modelica: unhandled statement: {stmt}"
    )))
}

/// The statements of a nested block, each indented one level deeper and
/// followed by a newline (empty for an empty block).
fn render_block(stmts: &Value, cfg: &ExprConfig, indent: &str) -> RenderResult {
    let body = render_statements(stmts, cfg, &format!("{indent}  "))?;
    Ok(if body.is_empty() {
        body
    } else {
        format!("{body}\n")
    })
}

/// `for i in r loop ... end for;`, one nested loop per iterator.
fn render_for(for_stmt: &Value, cfg: &ExprConfig, indent: &str) -> RenderResult {
    let indices = get_field(for_stmt, "indices")?;
    let count = indices
        .len()
        .ok_or_else(|| render_err("for statement indices is not a sequence"))?;
    if count == 0 {
        return Err(render_err("for statement has no iterator"));
    }
    let mut headers = Vec::with_capacity(count);
    for i in 0..count {
        let index = indices.get_item(&Value::from(i))?;
        let ident = get_field(&index, "ident")?;
        let range = render_expression(&get_field(&index, "range")?, cfg)?;
        let pad = "  ".repeat(i);
        headers.push(format!("{indent}{pad}for {ident} in {range} loop\n"));
    }
    let inner = format!("{indent}{}", "  ".repeat(count - 1));
    let body = render_block(&get_field(for_stmt, "equations")?, cfg, &inner)?;
    let mut out = headers.concat();
    out.push_str(&body);
    for i in (0..count).rev() {
        out.push_str(&format!("{indent}{}end for;", "  ".repeat(i)));
        if i > 0 {
            out.push('\n');
        }
    }
    Ok(out)
}

/// An `if`/`when` statement: guarded blocks, optional `else`, closing keyword.
fn render_branches(
    stmt: &Value,
    blocks_field: &str,
    [first, next]: [&str; 2],
    cfg: &ExprConfig,
    indent: &str,
) -> RenderResult {
    let blocks = get_field(stmt, blocks_field)?;
    let count = blocks
        .len()
        .ok_or_else(|| render_err(format!("{first} statement blocks are not a sequence")))?;
    let mut out = String::new();
    for i in 0..count {
        let block = blocks.get_item(&Value::from(i))?;
        let cond = render_expression(&get_field(&block, "cond")?, cfg)?;
        let keyword = if i == 0 { first } else { next };
        out.push_str(&format!("{indent}{keyword} {cond} then\n"));
        out.push_str(&render_block(&get_field(&block, "stmts")?, cfg, indent)?);
    }
    if let Ok(else_block) = get_field(stmt, "else_block")
        && !else_block.is_none()
    {
        out.push_str(&format!("{indent}else\n"));
        out.push_str(&render_block(&else_block, cfg, indent)?);
    }
    out.push_str(&format!("{indent}end {first};"));
    Ok(out)
}

/// `f(a);`, `y := f(a);` or `(y1, , y3) := f(a);` (MLS §11.2.1.1).
fn render_call(call: &Value, cfg: &ExprConfig, indent: &str) -> RenderResult {
    let raw_name = render_serialized_name(&get_field(call, "comp")?);
    let mut name = super::emitted_symbol(&raw_name, cfg)?;
    if cfg
        .quote_functions
        .as_ref()
        .is_some_and(|functions| super::names_contain(functions, &raw_name))
    {
        name = super::quote_identifier(&name);
    }
    let args = render_args(call, cfg)?;
    let outputs = get_field(call, "outputs").ok();
    let count = outputs.as_ref().and_then(Value::len).unwrap_or(0);
    let mut targets = Vec::with_capacity(count);
    for i in 0..count {
        let Some(output) = outputs.as_ref().map(|o| o.get_item(&Value::from(i))) else {
            break;
        };
        let output = output?;
        targets.push(if output.is_none() {
            String::new()
        } else {
            render_component_ref(&output, cfg)?
        });
    }
    Ok(match targets.as_slice() {
        [] => format!("{indent}{name}({args});"),
        [single] => format!("{indent}{single} := {name}({args});"),
        many => format!("{indent}({}) := {name}({args});", many.join(", ")),
    })
}

/// A component reference: the dotted path is one quoted identifier (the
/// declared name), its trailing subscripts stay subscripts (`'y'[i]`).
fn render_component_ref(comp: &Value, cfg: &ExprConfig) -> RenderResult {
    if let Some(name) = comp.as_str() {
        return super::emitted_variable_symbol(name, cfg);
    }
    let parts = get_field(comp, "parts")?;
    let count = parts
        .len()
        .ok_or_else(|| render_err("component reference parts are not a sequence"))?;
    let mut idents = Vec::with_capacity(count);
    let mut subscripts = String::new();
    for i in 0..count {
        let part = parts.get_item(&Value::from(i))?;
        let ident = get_field(&part, "ident")?;
        let subs = render_part_subscripts(&part, cfg)?;
        if !subscripts.is_empty() {
            return Err(render_err(format!(
                "base-modelica: subscripted record path `{}` in an algorithm is unsupported",
                idents.join(".")
            )));
        }
        let ident = ident.as_str().ok_or_else(|| {
            render_err(format!(
                "component reference part ident {ident} is not a name"
            ))
        })?;
        idents.push(ident.to_string());
        subscripts = subs;
    }
    let path = idents.join(".");
    let constant = subscripts.split(", ").all(|sub| sub.parse::<i64>().is_ok());
    if cfg.scalarize && !subscripts.is_empty() && constant {
        let element = format!("{path}[{}]", subscripts.replace(' ', ""));
        return Ok(super::quote_identifier(&element));
    }
    let base = super::emitted_variable_symbol(&path, cfg)?;
    Ok(if subscripts.is_empty() {
        base
    } else {
        format!("{base}[{subscripts}]")
    })
}

fn render_part_subscripts(part: &Value, cfg: &ExprConfig) -> RenderResult {
    let Ok(subs) = get_field(part, "subs") else {
        return Ok(String::new());
    };
    let count = subs.len().unwrap_or(0);
    let mut rendered = Vec::with_capacity(count);
    for i in 0..count {
        let sub = subs.get_item(&Value::from(i))?;
        rendered.push(super::render_expr::render_subscript(&sub, cfg)?);
    }
    Ok(rendered.join(", "))
}

/// `quote_ident(name)`: `name` as a quoted identifier (`R1.p.v` -> `'R1.p.v'`).
pub(super) fn quote_ident_function(name: Value) -> RenderResult {
    let name = name
        .as_str()
        .ok_or_else(|| render_err(format!("quote_ident expects a string, got {name}")))?;
    Ok(super::quote_identifier(name))
}

/// `is_builtin_alias(name)`: whether every call of function `name` renders as
/// a built-in operator (`Modelica.Math.sin` -> `sin`), so the package does
/// not need to declare it.
pub(super) fn is_builtin_alias_function(name: Value) -> bool {
    name.as_str()
        .is_some_and(|name| super::render_expr::resolve_modelica_math_function(name).is_some())
}

/// `scalar_indices(dims)`: every element subscript of shape `dims` in
/// row-major order, as comma-joined one-based indices (`[2, 2]` ->
/// `["1,1", "1,2", "2,1", "2,2"]`).
pub(super) fn scalar_indices_function(dims: Value) -> Result<Value, minijinja::Error> {
    let mut extents = Vec::new();
    for dim in dims.try_iter()? {
        let extent = dim
            .as_i64()
            .filter(|extent| *extent >= 0)
            .ok_or_else(|| render_err(format!("scalar_indices: dimension {dim} is not static")))?;
        extents.push(extent);
    }
    let mut rows: Vec<Vec<i64>> = vec![Vec::new()];
    for extent in extents {
        rows = rows
            .into_iter()
            .flat_map(|row| {
                (1..=extent).map(move |index| {
                    let mut next = row.clone();
                    next.push(index);
                    next
                })
            })
            .collect();
    }
    Ok(Value::from_serialize(
        rows.iter()
            .map(|row| row.iter().map(i64::to_string).collect::<Vec<_>>().join(","))
            .collect::<Vec<_>>(),
    ))
}

/// `array_element(expr, index, arrays)`: element `index` (`"2,1"`) of an
/// array-valued attribute or binding. An array literal is indexed, an
/// unsubscripted reference to an array in `arrays` (the declared variables)
/// gains the subscripts, and a scalar applies to every element (`each`).
pub(super) fn array_element_function(
    expr: Value,
    index: Value,
    arrays: Value,
) -> Result<Value, minijinja::Error> {
    let index = index
        .as_str()
        .ok_or_else(|| render_err(format!("array_element: index {index} is not a string")))?
        .to_string();
    let mut subscripts = Vec::new();
    for part in index.split(',') {
        subscripts.push(part.parse::<usize>().map_err(|_| {
            render_err(format!("array_element: `{index}` is not an element index"))
        })?);
    }
    let mut current = expr.clone();
    for (depth, position) in subscripts.iter().enumerate() {
        if let Ok(array) = get_field(&current, "Array") {
            current = get_field(&array, "elements")?.get_item(&Value::from(position - 1))?;
            if current.is_undefined() {
                return Err(render_err(format!(
                    "array_element: index {index} is outside the array literal"
                )));
            }
            continue;
        }
        if depth > 0 {
            return Err(render_err(format!(
                "array_element: cannot select element {index} of a ragged array value"
            )));
        }
        return element_of_scalar_or_array_ref(&current, &subscripts, &arrays);
    }
    Ok(current)
}

fn element_of_scalar_or_array_ref(
    expr: &Value,
    subscripts: &[usize],
    arrays: &Value,
) -> Result<Value, minijinja::Error> {
    let Ok(var_ref) = get_field(expr, "VarRef") else {
        if is_scalar_valued(expr) {
            return Ok(expr.clone());
        }
        return Err(render_err(format!(
            "array_element: cannot scalarize array-valued expression {expr}"
        )));
    };
    let name = render_serialized_name(&get_field(&var_ref, "name")?);
    let has_subscripts = get_field(&var_ref, "subscripts")
        .ok()
        .and_then(|subs| subs.len())
        .is_some_and(|len| len > 0);
    let is_array = arrays
        .get_item(&Value::from(name.as_str()))
        .ok()
        .and_then(|var| var.get_attr("dims").ok())
        .and_then(|dims| dims.len())
        .is_some_and(|len| len > 0);
    if has_subscripts || !is_array {
        return Ok(expr.clone());
    }
    let subscripts = subscripts
        .iter()
        .map(|index| serde_json::json!({ "Index": index }))
        .collect::<Vec<_>>();
    Ok(Value::from_serialize(serde_json::json!({
        "VarRef": { "name": name, "subscripts": subscripts }
    })))
}

/// Literals and scalar operators broadcast over every element; anything else
/// (a call, a comprehension, a range) may be array-valued.
fn is_scalar_valued(expr: &Value) -> bool {
    get_field(expr, "Literal").is_ok()
        || get_field(expr, "Unary")
            .and_then(|unary| get_field(&unary, "rhs"))
            .is_ok_and(|rhs| is_scalar_valued(&rhs))
}
