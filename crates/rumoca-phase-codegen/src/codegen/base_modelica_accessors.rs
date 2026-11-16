//! Member access on a record-valued expression for the Base Modelica targets.
//!
//! Flat reads a member straight off a call that returns a record
//! (`Frames.nullRotation().T`, `Complex.'*'.multiply(a, b).re`, and the field
//! equations `record_field_equations` writes for `v = f(...)`), or off a
//! record-valued `if` expression (`(if c then f(a) else Complex(0, 0)).re`).
//! Neither MCP-0031 nor MLS 3.6 has that form: a `primary` is a component
//! reference optionally followed by call arguments, with nothing after the
//! call (MLS 3.7-dev only adds `(f(x)).re`, which MCP-0031 does not take
//! over), and "a member of a record can only be accessed through an instance
//! of the record" (MCP-0031 differences.md).
//!
//! A read off an `if` moves into each arm, and a read off a record
//! constructor call is the argument for that field (`lower_member_read`).
//! Each (function, member path) pair read off any other call gets a generated
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
use super::render_expr::get_field;
use crate::errors::render_err;
use minijinja::Value;
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

type Accessors = Result<Value, minijinja::Error>;

/// The generated accessor for member `path` of the result of `function`.
pub(super) fn accessor_name(function: &str, path: &[String]) -> String {
    format!("-{function}().{}", path.join("."))
}

/// The root and member path of a member-access chain (`e.a.b` -> (`e`,
/// `[a, b]`)), from the contents of a `FieldAccess` node.
pub(super) fn member_chain(field_access: &Value) -> Option<(Value, Vec<String>)> {
    let field = get_field(field_access, "field").ok()?.as_str()?.to_string();
    let base = get_field(field_access, "base").ok()?;
    let (root, mut path) = match get_field(&base, "FieldAccess") {
        Ok(inner) => member_chain(&inner)?,
        Err(_) => (base, Vec::new()),
    };
    path.push(field);
    Some((root, path))
}

/// The record fields of constructor `name`, in declaration order, read off
/// a template value of the model's functions.
pub(super) fn constructor_fields(functions: &Value, name: &str) -> Option<Vec<String>> {
    let function = get_field(functions, name).ok()?;
    if !get_field(&function, "is_constructor").ok()?.is_true() {
        return None;
    }
    get_field(&function, "inputs")
        .ok()?
        .try_iter()
        .ok()?
        .map(|input| Some(get_field(&input, "name").ok()?.as_str()?.to_string()))
        .collect()
}

/// `root.path` without reading a member off an `if` expression or a record
/// constructor call: the read moves into each `if` arm, and a constructor's
/// member is its argument for that field (`Complex(a, b).re` -> `a`). None
/// when the root is neither, or the constructor call leaves the field out.
pub(super) fn lower_member_read(
    root: &Json,
    path: &[String],
    fields_of: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Option<Json> {
    if let Some(if_expr) = root.get("If") {
        let mut lowered = if_expr.clone();
        for branch in lowered["branches"].as_array_mut()? {
            branch[1] = member_read(&branch[1], path, fields_of);
        }
        lowered["else_branch"] = member_read(&if_expr["else_branch"], path, fields_of);
        return Some(json!({ "If": lowered }));
    }
    let call = root.get("FunctionCall")?;
    if call["is_constructor"].as_bool() != Some(true) {
        return None;
    }
    let position = fields_of(&reference_name(&call["name"])?)?
        .iter()
        .position(|field| *field == path[0])?;
    let argument = call["args"].as_array()?.get(position)?;
    Some(member_read(argument, &path[1..], fields_of))
}

/// `base.path` as an expression, lowered where `lower_member_read` can.
fn member_read(
    base: &Json,
    path: &[String],
    fields_of: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Json {
    if path.is_empty() {
        return base.clone();
    }
    lower_member_read(base, path, fields_of).unwrap_or_else(|| {
        path.iter().fold(
            base.clone(),
            |base, field| json!({ "FieldAccess": { "base": base, "field": field } }),
        )
    })
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
    let flat_json = to_json(&flat)?;
    let functions_json = &flat_json["functions"];
    let mut uses = BTreeSet::new();
    collect(&flat_json, functions_json, &mut uses);
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
                collect(row, functions_json, &mut uses);
                continue;
            };
            let mut expanded = Vec::new();
            expand(equation, &layout, &mut expanded)?;
            for eq in &expanded {
                collect(eq, functions_json, &mut uses);
            }
        }
    }
    let accessors = uses
        .into_iter()
        .map(|(function, path)| declaration(functions_json, function, path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::from_serialize(accessors))
}

/// Every (function, member path) a `FieldAccess` chain in `node` reads off a
/// call, after `lower_member_read`. A chain is one use: its inner prefixes
/// (`f(x).a` in `f(x).a.b`) are not read on their own.
fn collect(node: &Json, functions: &Json, uses: &mut BTreeSet<(String, Vec<String>)>) {
    match node {
        Json::Object(map) => {
            if let Some(access) = map.get("FieldAccess")
                && let Some((root, path)) = json_member_chain(access)
            {
                let fields_of = |name: &str| json_constructor_fields(functions, name);
                if let Some(lowered) = lower_member_read(root, &path, &fields_of) {
                    return collect(&lowered, functions, uses);
                }
                if let Some(call) = root.get("FunctionCall")
                    && let Some(function) = reference_name(&call["name"])
                {
                    uses.insert((function, path));
                    return collect(call, functions, uses);
                }
            }
            map.values()
                .for_each(|child| collect(child, functions, uses));
        }
        Json::Array(items) => items
            .iter()
            .for_each(|child| collect(child, functions, uses)),
        _ => {}
    }
}

fn json_member_chain(access: &Json) -> Option<(&Json, Vec<String>)> {
    let field = access["field"].as_str()?.to_string();
    let base = &access["base"];
    let (root, mut path) = match base.get("FieldAccess") {
        Some(inner) => json_member_chain(inner)?,
        None => (base, Vec::new()),
    };
    path.push(field);
    Some((root, path))
}

fn json_constructor_fields(functions: &Json, name: &str) -> Option<Vec<String>> {
    let function = functions.get(name)?;
    if function["is_constructor"].as_bool() != Some(true) {
        return None;
    }
    function["inputs"]
        .as_array()?
        .iter()
        .map(|input| Some(input["name"].as_str()?.to_string()))
        .collect()
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
        record = member["type_name"]
            .as_str()
            .ok_or_else(|| unresolved(&format!("member `{field}` has no declared type")))?
            .to_string();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessor_declaration_refuses_a_member_without_a_declared_type() {
        let functions = json!({
            "f": { "outputs": [{ "type_name": "R" }], "inputs": [], "pure": true },
            "R": { "is_constructor": true, "inputs": [{ "name": "q" }] },
        });
        let error = declaration(&functions, "f".into(), vec!["q".into()])
            .expect_err("a missing member type cannot become an empty type name");
        assert!(
            error.to_string().contains("has no declared type"),
            "{error}"
        );
    }
}
