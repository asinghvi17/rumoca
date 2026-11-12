//! Record-valued model equations for the Base Modelica targets.
//!
//! The model's record components are flattened to their fields (`'z.re'`,
//! `'z.im'`), which MCP-0031 permits, so a whole-record equation on one has no
//! Base Modelica spelling. MLS §8.3.1 defines a record equation as the
//! equations of its fields, which is what `record_field_equations(eq, ...)`
//! writes for the two value forms a field can be read off without a helper:
//!
//! * another flattened record of the same type (`a.i = b.i`), field by field;
//! * a record constructor call (`z = Complex(re, im)`), one argument per field
//!   in declaration order, a missing trailing argument taking the
//!   constructor's default;
//! * a call to a function returning the record (`v = Complex.'-'.subtract(a,
//!   b)`), each field read off the call (`'v.re' = 'Complex.\'-\'.subtract'(a,
//!   b).'re'`), the member access Flat already writes for a record-valued
//!   call nested in an expression.
//!
//! Any other value is left whole, and the renderer refuses it as before.

use crate::errors::render_err;
use minijinja::Value;
use serde_json::{Value as Json, json};

type Expansion<T> = Result<T, minijinja::Error>;

/// `record_field_equations(eq, record_instances, record_types, functions)`:
/// the field equations of a record-valued equation, or the equation itself.
pub(super) fn record_field_equations_function(
    eq: Value,
    instances: Value,
    types: Value,
    functions: Value,
) -> Expansion<Value> {
    let eq = serde_json::to_value(&eq)
        .map_err(|err| render_err(format!("base-modelica: unserializable equation: {err}")))?;
    let layout = Layout {
        instances: &instances,
        types: &types,
        functions: &functions,
    };
    let mut rows = Vec::new();
    expand(&eq, &layout, &mut rows)?;
    Ok(Value::from_serialize(rows))
}

struct Layout<'a> {
    instances: &'a Value,
    types: &'a Value,
    functions: &'a Value,
}

fn expand(eq: &Json, layout: &Layout<'_>, rows: &mut Vec<Json>) -> Expansion<()> {
    let Some(fields) = field_pairs(eq, layout)? else {
        rows.push(eq.clone());
        return Ok(());
    };
    for (target, value) in fields {
        let mut row = eq.clone();
        let binary = &mut row["residual"]["Binary"];
        binary["lhs"] = target;
        binary["rhs"] = value;
        row["scalar_count"] = json!(1);
        // A field that is itself a record expands again.
        expand(&row, layout, rows)?;
    }
    Ok(())
}

/// The (field target, field value) pairs of `eq`, when it equates a flattened
/// scalar record with a value whose fields can be named.
fn field_pairs(eq: &Json, layout: &Layout<'_>) -> Expansion<Option<Vec<(Json, Json)>>> {
    let Some(binary) = eq["residual"].get("Binary") else {
        return Ok(None);
    };
    if !matches!(binary["op"].as_str(), Some("Sub" | "SubElem")) {
        return Ok(None);
    }
    let Some(target) = record_ref(&binary["lhs"], layout) else {
        return Ok(None);
    };
    let Some(fields) = record_fields(&target, layout)? else {
        return Ok(None);
    };
    let value = &binary["rhs"];
    let values: Vec<Json> = if let Some(source) = record_ref(value, layout) {
        fields
            .iter()
            .map(|field| field_ref(&format!("{source}.{field}")))
            .collect()
    } else if let Some(call) = value.get("FunctionCall") {
        if call["is_constructor"].as_bool() == Some(true) {
            let Some(values) = constructor_arguments(call, fields.len(), layout) else {
                return Ok(None);
            };
            values
        } else {
            fields
                .iter()
                .map(|field| json!({ "FieldAccess": { "base": value, "field": field } }))
                .collect()
        }
    } else {
        return Ok(None);
    };
    Ok(Some(
        fields
            .iter()
            .map(|field| field_ref(&format!("{target}.{field}")))
            .zip(values)
            .collect(),
    ))
}

/// The name of an unsubscripted reference to a flattened scalar record.
fn record_ref(expr: &Json, layout: &Layout<'_>) -> Option<String> {
    let var_ref = expr.get("VarRef")?;
    if var_ref["subscripts"]
        .as_array()
        .is_some_and(|subs| !subs.is_empty())
    {
        return None;
    }
    let name = match &var_ref["name"] {
        Json::String(name) => name.clone(),
        name => name["name"].as_str()?.to_string(),
    };
    let instance = item(layout.instances, &Value::from(name.as_str()))?;
    instance_dims_empty(&instance).then_some(name)
}

fn instance_dims_empty(instance: &Value) -> bool {
    item(instance, &Value::from("dims")).is_none_or(|dims| dims.len() == Some(0))
}

/// The declared field names of the record type of instance `name`.
fn record_fields(name: &str, layout: &Layout<'_>) -> Expansion<Option<Vec<String>>> {
    let Some(instance) = item(layout.instances, &Value::from(name)) else {
        return Ok(None);
    };
    let Some(type_id) = item(&instance, &Value::from("type_def_id")) else {
        return Ok(None);
    };
    let Some(record) = item(layout.types, &type_id) else {
        return Ok(None);
    };
    let Some(fields) = item(&record, &Value::from("fields")) else {
        return Ok(None);
    };
    let mut names = Vec::new();
    for field in fields
        .try_iter()
        .map_err(|err| render_err(format!("record type fields are not a list: {err}")))?
    {
        let Some(name) = item(&field, &Value::from("name")) else {
            return Ok(None);
        };
        names.push(name.to_string());
    }
    Ok((!names.is_empty()).then_some(names))
}

/// One value per field from a constructor call: positional arguments, then
/// the constructor's defaults for the fields the call leaves out.
fn constructor_arguments(call: &Json, fields: usize, layout: &Layout<'_>) -> Option<Vec<Json>> {
    let args = call["args"].as_array()?;
    if args.len() > fields {
        return None;
    }
    let mut values = args.clone();
    if values.len() < fields {
        let name = match &call["name"] {
            Json::String(name) => name.clone(),
            name => name["name"].as_str()?.to_string(),
        };
        let function = item(layout.functions, &Value::from(name.as_str()))?;
        let inputs = item(&function, &Value::from("inputs"))?;
        for position in values.len()..fields {
            let input = inputs.get_item(&Value::from(position)).ok()?;
            let default = item(&input, &Value::from("default"))?;
            values.push(serde_json::to_value(&default).ok()?);
        }
    }
    Some(values)
}

fn field_ref(name: &str) -> Json {
    json!({ "VarRef": { "name": name, "subscripts": [] } })
}

/// `value[key]` when present and not none.
fn item(value: &Value, key: &Value) -> Option<Value> {
    value
        .get_item(key)
        .ok()
        .filter(|item| !item.is_undefined() && !item.is_none())
}
