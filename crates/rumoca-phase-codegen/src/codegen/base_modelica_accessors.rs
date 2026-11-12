//! Member access on a record-valued call for the Base Modelica targets.
//!
//! Flat reads a member straight off a call that returns a record
//! (`Frames.nullRotation().T`, `Complex.'*'.multiply(a, b).re`, and the field
//! equations `record_field_equations` writes for `v = f(...)`). Neither
//! MCP-0031 nor MLS 3.6 has that form: a `primary` is a component reference
//! optionally followed by call arguments, with nothing after the call (MLS
//! 3.7-dev only adds `(f(x)).re`, which MCP-0031 does not take over), and
//! "a member of a record can only be accessed through an instance of the
//! record" (MCP-0031 differences.md).
//!
//! So each (function, member path) pair read this way gets a generated
//! accessor function with the function's inputs that stores the call in a
//! record local and returns the member:
//!
//! ```text
//! function '-f().re' "Automatically generated accessor for member re of the result of f"
//!   input Real 'a';
//!   output Real '-member';
//!   'R' '-record';
//! algorithm
//!   '-record' := 'f'('a');
//!   '-member' := '-record'.'re';
//! end '-f().re';
//! ```
//!
//! and `f(x).re` renders as `'-f().re'(x)`. The leading hyphen puts the name
//! in the part of the namespace MCP-0031 leaves to generated names, and `()`
//! cannot occur in a source name, so it collides with neither source
//! functions nor the generated constructors (`'-R'`). The rewrite is local to
//! the expression, so it holds wherever the access occurs (equations,
//! bindings, attributes, conditions, function bodies) and keeps the call's
//! evaluation exactly where it was.

use super::base_modelica_records::{Layout, expand};
use crate::errors::render_err;
use minijinja::Value;
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

type Accessors = Result<Value, minijinja::Error>;

/// The generated accessor for member `path` of the result of `function`.
pub(super) fn accessor_name(function: &str, path: &[String]) -> String {
    format!("-{function}().{}", path.join("."))
}

/// The call and member path of a member-access chain rooted at a function
/// call (`f(x).a.b` -> (`f(x)`, `[a, b]`)), from the template value of a
/// `FieldAccess` node's contents.
pub(super) fn call_member_chain(field_access: &Value) -> Option<(Value, Vec<String>)> {
    let field = field_access.get_attr("field").ok()?.as_str()?.to_string();
    let base = field_access.get_attr("base").ok()?;
    if let Ok(call) = base.get_attr("FunctionCall")
        && !call.is_undefined()
    {
        return Some((call, vec![field]));
    }
    let inner = base.get_attr("FieldAccess").ok()?;
    if inner.is_undefined() {
        return None;
    }
    let (call, mut path) = call_member_chain(&inner)?;
    path.push(field);
    Some((call, path))
}

/// `record_field_accessors(flat, flat_equation_rows)`: one entry per member
/// read off a call anywhere in the model, including the field equations
/// `record_field_equations` writes, in name order:
/// `{name, function, call, path, inputs, member, record}` where `call` is the
/// called function's emitted name, `member` and `record` are the output and
/// local declarations (function parameters), and `inputs` the function's.
pub(super) fn record_field_accessors_function(flat: Value, rows: Value) -> Accessors {
    let to_json = |value: &Value| {
        serde_json::to_value(value)
            .map_err(|err| render_err(format!("base-modelica: unserializable model: {err}")))
    };
    let mut uses = BTreeSet::new();
    collect(&to_json(&flat)?, &mut uses);
    let instances = flat.get_attr("record_instances")?;
    let types = flat.get_attr("record_types")?;
    let functions = flat.get_attr("functions")?;
    let layout = Layout {
        instances: &instances,
        types: &types,
        functions: &functions,
    };
    let rows = to_json(&rows)?;
    for partition in ["regular", "initial"] {
        for row in rows[partition].as_array().into_iter().flatten() {
            let Some(equation) = row.get("equation") else {
                collect(row, &mut uses);
                continue;
            };
            let mut expanded = Vec::new();
            expand(equation, &layout, &mut expanded)?;
            for eq in &expanded {
                collect(eq, &mut uses);
            }
        }
    }
    let functions = to_json(&functions)?;
    let accessors = uses
        .into_iter()
        .map(|(function, path)| declaration(&functions, function, path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::from_serialize(accessors))
}

/// Every (function, member path) a `FieldAccess` chain in `node` reads off a
/// call. A chain is one use: its inner prefixes (`f(x).a` in `f(x).a.b`) are
/// not read on their own.
fn collect(node: &Json, uses: &mut BTreeSet<(String, Vec<String>)>) {
    match node {
        Json::Object(map) => {
            if let Some(access) = map.get("FieldAccess")
                && let Some((call, path)) = json_call_member_chain(access)
                && let Some(function) = reference_name(&call["name"])
            {
                uses.insert((function, path));
                return collect(call, uses);
            }
            map.values().for_each(|child| collect(child, uses));
        }
        Json::Array(items) => items.iter().for_each(|child| collect(child, uses)),
        _ => {}
    }
}

fn json_call_member_chain(access: &Json) -> Option<(&Json, Vec<String>)> {
    let field = access["field"].as_str()?.to_string();
    let base = &access["base"];
    if let Some(call) = base.get("FunctionCall") {
        return Some((call, vec![field]));
    }
    let (call, mut path) = json_call_member_chain(base.get("FieldAccess")?)?;
    path.push(field);
    Some((call, path))
}

fn reference_name(name: &Json) -> Option<String> {
    match name {
        Json::String(name) => Some(name.clone()),
        name => reference_name(name.get("name")?),
    }
}

/// The accessor entry for member `path` of `function`'s (first) result: the
/// member's declared type is read off the record constructors, as
/// `record_decl` declares the record classes.
fn declaration(
    functions: &Json,
    function: String,
    path: Vec<String>,
) -> Result<Json, minijinja::Error> {
    let unresolved = |what: &str| {
        render_err(format!(
            "base-modelica: cannot declare an accessor for member `{}` of the result of `{function}`: {what}",
            path.join(".")
        ))
    };
    let callee = functions
        .get(&function)
        .ok_or_else(|| unresolved("the function is not in the model"))?;
    let constructor = callee["is_constructor"].as_bool() == Some(true);
    let mut record = if constructor {
        function.clone()
    } else {
        callee["outputs"][0]["type_name"]
            .as_str()
            .ok_or_else(|| unresolved("it has no output"))?
            .to_string()
    };
    let result_record = record.clone();
    let mut member = Json::Null;
    for field in &path {
        let fields = functions
            .get(&record)
            .filter(|ctor| ctor["is_constructor"].as_bool() == Some(true))
            .and_then(|ctor| ctor["inputs"].as_array())
            .ok_or_else(|| unresolved(&format!("`{record}` is not a record with a constructor")))?;
        member = fields
            .iter()
            .find(|input| input["name"].as_str() == Some(field.as_str()))
            .cloned()
            .ok_or_else(|| unresolved(&format!("`{record}` has no member `{field}`")))?;
        record = member["type_name"].as_str().unwrap_or_default().to_string();
    }
    member["name"] = json!("-member");
    member["default"] = Json::Null;
    member["description"] = Json::Null;
    Ok(json!({
        "name": accessor_name(&function, &path),
        "function": function,
        "call": if constructor { format!("-{function}") } else { function.clone() },
        "path": path,
        "pure": callee["pure"].as_bool().unwrap_or(true),
        "inputs": callee["inputs"],
        "member": member,
        "record": {
            "name": "-record",
            "type_name": result_record,
            "type_class": "Record",
            "shape_expr": [],
            "effective_type": { "dimensions": [] },
            "default": null,
            "description": null,
        },
    }))
}
