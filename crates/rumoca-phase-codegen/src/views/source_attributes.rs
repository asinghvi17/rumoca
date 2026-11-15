//! Typed read-only source-attribute rank facts from the canonical Flat owner.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use minijinja::{Error, ErrorKind, Value};
use rumoca_ir_flat::{
    Model,
    source_view::{SourceAttributeKind, SourceAttributeValue},
};
use serde::Serialize;

#[derive(Serialize)]
struct AttributeRank {
    minimum_rank: usize,
    each: bool,
}

type AttributeRanks = BTreeMap<&'static str, Result<Option<AttributeRank>, String>>;

/// `flat_source_attribute_rank(instance_id, kind)` returns `{minimum_rank, each}`.
/// Attribute failures are deferred independently until that attribute is queried.
pub(crate) fn context_value(model: &Model) -> Value {
    let shapes = model
        .source_attributes()
        .map_err(|error| format!("{error:?}"))
        .map(|catalog| {
            model
                .variables
                .values()
                .map(|variable| {
                    let attributes = [
                        ("start", SourceAttributeKind::Start),
                        ("min", SourceAttributeKind::Min),
                        ("max", SourceAttributeKind::Max),
                        ("nominal", SourceAttributeKind::Nominal),
                    ]
                    .into_iter()
                    .map(|(key, kind)| {
                        let value = catalog
                            .attribute(variable.instance_id, kind)
                            .map(|value| value.map(attribute_shape))
                            .map_err(|error| error.to_string());
                        (key, value)
                    })
                    .collect::<AttributeRanks>();
                    (variable.instance_id.index(), attributes)
                })
                .collect::<BTreeMap<_, _>>()
        });
    Value::from_function(move |instance: u32, kind: String| -> Result<Value, Error> {
        let variables = shapes.as_ref().map_err(refusal)?;
        let shape = variables
            .get(&instance)
            .and_then(|attributes| attributes.get(kind.as_str()))
            .ok_or_else(|| refusal("missing source attribute occurrence or kind"))?
            .as_ref()
            .map_err(refusal)?
            .as_ref()
            .ok_or_else(|| refusal("missing source attribute value"))?;
        Ok(Value::from_serialize(shape))
    })
}

fn attribute_shape(value: SourceAttributeValue<'_>) -> AttributeRank {
    AttributeRank {
        minimum_rank: value.minimum_rank(),
        each: value.each(),
    }
}

fn refusal(message: impl AsRef<str>) -> Error {
    Error::new(ErrorKind::InvalidOperation, message.as_ref().to_owned())
}
