//! Typed read-only source-attribute shape facts from the canonical Flat owner.

use std::collections::BTreeMap;

use minijinja::{Error, ErrorKind, Value};
use rumoca_ir_flat::{
    Model,
    source_view::{SourceAttributeKind, SourceAttributeValue},
};
use serde::Serialize;

#[derive(Serialize)]
struct AttributeShape {
    dimensions: Vec<i64>,
    each: bool,
}

type VariableShapes = Result<BTreeMap<&'static str, AttributeShape>, String>;

/// `flat_source_attribute_shape(instance_id, kind)` returns `{dimensions, each}`.
/// It copies canonical facts once; queries take logarithmic lookup time
/// and refuse invalid models or unknown shapes.
/// Errors are deferred until queried so unrelated Flat consumers stay independent.
pub(crate) fn context_value(model: &Model) -> Value {
    let shapes = model
        .source_attributes()
        .map_err(|error| format!("{error:?}"))
        .map(|catalog| {
            model
                .variables
                .values()
                .map(|variable| {
                    let values = catalog
                        .variable(variable.instance_id)
                        .map(|values| values.into_iter().map(attribute_shape).collect())
                        .map_err(|error| error.to_string());
                    (variable.instance_id.index(), values)
                })
                .collect::<BTreeMap<u32, VariableShapes>>()
        });
    Value::from_function(move |instance: u32, kind: String| -> Result<Value, Error> {
        let variables = shapes.as_ref().map_err(refusal)?;
        let attributes = variables
            .get(&instance)
            .ok_or_else(|| refusal("missing source attribute occurrence"))?
            .as_ref()
            .map_err(refusal)?;
        let shape = attributes
            .get(kind.as_str())
            .ok_or_else(|| refusal("missing source attribute kind"))?;
        Ok(Value::from_serialize(shape))
    })
}

fn attribute_shape(value: SourceAttributeValue<'_>) -> (&'static str, AttributeShape) {
    let kind = match value.kind() {
        SourceAttributeKind::Start => "start",
        SourceAttributeKind::Min => "min",
        SourceAttributeKind::Max => "max",
        SourceAttributeKind::Nominal => "nominal",
    };
    (
        kind,
        AttributeShape {
            dimensions: value.dimensions().to_vec(),
            each: value.each(),
        },
    )
}

fn refusal(message: impl AsRef<str>) -> Error {
    Error::new(ErrorKind::InvalidOperation, message.as_ref().to_owned())
}
