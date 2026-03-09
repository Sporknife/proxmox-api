use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::generator::{
    AdditionalProperties, BoundedIntegerDef, BoundedNumberDef, BoundedStringDef, FieldDef,
    PrimitiveTypeDef, TypeDef, clean_doc,
};

use super::{Format, KnownFormat, Optional, Output};

fn deserialize_optional_i128<'de, D>(deserializer: D) -> Result<Option<i128>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Option<i128>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "an integer, a string encoding an integer, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(Visitor)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v as i128))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v as i128))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // Lenient: return None for non-numeric strings (descriptive defaults)
            Ok(v.parse::<i128>().ok())
        }
    }

    deserializer.deserialize_option(Visitor)
}

fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Option<f64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a number, a string encoding a number, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(Visitor)
        }

        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v as f64))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v as f64))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // Lenient: return None for non-numeric strings (descriptive defaults)
            Ok(v.parse::<f64>().ok())
        }
    }

    deserializer.deserialize_option(Visitor)
}

fn deserialize_optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Option<bool>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a boolean, 0/1, a string encoding those values, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(Visitor)
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v))
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(match v {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(match v {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let value = match v.trim().to_ascii_lowercase().as_str() {
                "0" | "false" => Some(false),
                "1" | "true" => Some(true),
                _ => None,
            };
            Ok(value)
        }
    }

    deserializer.deserialize_option(Visitor)
}

fn extract_string_constraints(ty: &TypeKind, field_name: &str) -> Option<BoundedStringDef> {
    let TypeKind::String {
        max_length,
        min_length,
        pattern,
        enum_values: None,
        default,
    } = ty
    else {
        return None;
    };

    let has_constraints = pattern.is_some() || min_length.is_some() || max_length.is_some();
    if !has_constraints {
        return None;
    }

    let pattern = pattern.as_ref().map(|pattern| pattern.to_string());

    let name = crate::name_to_ident(&format!("{}Str", field_name));

    Some(BoundedStringDef {
        name,
        min_length: min_length.map(|min| min as usize),
        max_length: max_length.map(|max| max as usize),
        pattern,
        default_val: default
            .as_ref()
            .map(|cow_str: &Cow<'_, str>| String::from(cow_str.clone())),
    })
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Type<'a> {
    #[serde(flatten, borrow, default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<TypeKind<'a>>,

    #[serde(
        rename = "typetext",
        borrow,
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub type_text: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Optional::is_empty")]
    pub optional: Optional,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose_description: Option<Cow<'a, str>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format<'a>>,
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub format_description: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_key: Option<u32>,
    // Another field in the parent object is required
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<Cow<'a, str>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<Cow<'a, str>>,
    // This type is an alias for a field in the parent object
    #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<Cow<'a, str>>,
    // This type is an alias for a field in the parent object.
    #[serde(
        alias = "keyAlias",
        borrow,
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub key_alias: Option<Cow<'a, str>>,
}

impl Type<'_> {
    pub fn doc(&self) -> impl Iterator<Item = String> + '_ {
        self.description
            .as_ref()
            .into_iter()
            .chain(self.verbose_description.as_ref())
            .flat_map(clean_doc)
    }

    pub fn type_def(&self, field_name: &str, struct_suffix: &str) -> Option<Output> {
        // TODO: report that "command" only has format, and no type
        if self.format == Some(Format::Kind(KnownFormat::String)) && self.ty.is_none() {
            return Some(Output::bare_def(TypeDef::Primitive(
                PrimitiveTypeDef::String,
            )));
        }

        let output_type = if let Some(ty) = self.ty.as_ref() {
            match ty {
                TypeKind::String {
                    enum_values,
                    default,
                    ..
                } => {
                    if let Some(enum_values) = enum_values {
                        let default = if enum_values.iter().any(|v| Some(v) == default.as_ref()) {
                            default.as_ref().map(Cow::to_string)
                        } else {
                            None
                        };

                        let enum_values: BTreeSet<_> = enum_values
                            .iter()
                            .map(Cow::to_string)
                            .chain(default.clone())
                            .collect();
                        let no_derives = Option::<&str>::None;

                        // TODO: Fix this ugly hack...
                        // Would be nice to find a better workaround that doesn't require one-offs
                        // and also doesn't require global state...
                        let name = if enum_values.contains("cdrom-image-ignored") {
                            crate::name_to_ident("warning_type")
                        } else {
                            crate::name_to_ident(field_name)
                        };

                        TypeDef::new_enum(name, no_derives, enum_values, default, self.doc())
                    } else if let Some(bounded) = extract_string_constraints(ty, field_name) {
                        TypeDef::BoundedString(bounded)
                    } else {
                        TypeDef::Primitive(PrimitiveTypeDef::String)
                    }
                }
                TypeKind::Number {
                    minimum,
                    maximum,
                    default,
                } => match (*minimum, *maximum, *default) {
                    (None, None, None | Some(0.0)) => TypeDef::Primitive(PrimitiveTypeDef::Number),
                    _ => TypeDef::BoundedNumber(BoundedNumberDef {
                        name: crate::name_to_ident(&format!("{}Num", field_name)),
                        min: *minimum,
                        max: *maximum,
                        default: *default,
                    }),
                },
                TypeKind::Integer {
                    minimum,
                    maximum,
                    default,
                } => match (*minimum, *maximum, *default) {
                    (None, None, None | Some(0)) => TypeDef::Primitive(PrimitiveTypeDef::Integer),
                    _ => TypeDef::BoundedInteger(BoundedIntegerDef {
                        name: crate::name_to_ident(&format!("{}Int", field_name)),
                        min: *minimum,
                        max: *maximum,
                        default: *default,
                    }),
                },
                TypeKind::Boolean { .. } => TypeDef::Primitive(PrimitiveTypeDef::Boolean),
                TypeKind::Array { items } => {
                    let mut output =
                        items.type_def(field_name, &format!("{struct_suffix}Items"))?;
                    let def = output.def?.clone();
                    output.module_defs.push(def.clone());
                    output.def = Some(TypeDef::Array(Box::new(def)));

                    return Some(output);
                }
                TypeKind::Object {
                    properties,
                    additional_properties,
                } => {
                    let mut final_output = Output::new();

                    let field_name = crate::name_to_ident(field_name);
                    let suffix = crate::name_to_ident(struct_suffix);
                    let struct_name = format!("{field_name}{suffix}");
                    let mut all_external = Vec::new();

                    let (additional_props, ext) =
                        additional_properties.as_additional_properties(struct_suffix);

                    all_external.extend(ext);

                    if let Some(props) = properties {
                        let fields: Vec<_> = props
                            .iter()
                            .filter_map(|(original_name, ty)| {
                                let field_name = crate::name_to_ident(original_name);
                                let output = ty.type_def(
                                    &field_name,
                                    &format!("{struct_suffix}{field_name}"),
                                )?;
                                let inner = output.def.as_ref()?.clone();
                                final_output.absorb(output);

                                let doc = ty.doc();
                                let optional = ty.optional.get();

                                let field =
                                    FieldDef::new(original_name.to_string(), inner, optional, doc);

                                if let Some(numbered_items) = field.numbered_items() {
                                    final_output.module_defs.push(TypeDef::NumberedItems(
                                        Box::new(numbered_items.clone()),
                                    ));
                                }

                                Some(field)
                            })
                            .collect();

                        let def = TypeDef::new_struct(struct_name, fields, additional_props);
                        final_output.def = Some(def);

                        return Some(final_output);
                    } else if !additional_props.is_none() {
                        TypeDef::new_struct(struct_name, Vec::new(), additional_props)
                    } else {
                        return None;
                    }
                }
            }
        } else {
            return None;
        };

        let def = if let (Some(fallback), Some(Format::Kind(format))) =
            (output_type.primitive(), &self.format)
        {
            let format = match format {
                KnownFormat::MacAddr { .. } => {
                    let allow_ig_bit = self
                        .verbose_description
                        .as_ref()
                        .map(|v| !v.contains("the I/G (Individual/Group) bit not set"))
                        .unwrap_or(true);
                    KnownFormat::MacAddr(allow_ig_bit)
                }
                format => *format,
            };

            TypeDef::KnownType { format, fallback }
        } else {
            output_type
        };

        Some(Output::bare_def(def))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum IntOrTy<'a> {
    Int(u32),
    #[serde(borrow)]
    Ty(Box<Type<'a>>),
}

impl IntOrTy<'_> {
    pub fn as_additional_properties(
        &self,
        struct_suffix: &str,
    ) -> (AdditionalProperties, Vec<TypeDef>) {
        match self {
            IntOrTy::Int(1) => (AdditionalProperties::Untyped, Vec::new()),
            IntOrTy::Ty(ty) => {
                let output = ty.type_def("additional_properties", struct_suffix).unwrap();
                (
                    AdditionalProperties::Type(Box::new(output.def.unwrap())),
                    output.module_defs,
                )
            }
            _ => (AdditionalProperties::None, Vec::new()),
        }
    }

    pub fn is_unset(&self) -> bool {
        matches!(self, Self::Int(0))
    }
}

impl Default for IntOrTy<'_> {
    fn default() -> Self {
        Self::Int(1)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TypeKind<'a> {
    String {
        #[serde(rename = "maxLength", default, skip_serializing_if = "Option::is_none")]
        max_length: Option<u32>,
        #[serde(rename = "minLength", default, skip_serializing_if = "Option::is_none")]
        min_length: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pattern: Option<Cow<'a, str>>,

        // If it's an enum
        #[serde(rename = "enum", default, skip_serializing_if = "Option::is_none")]
        enum_values: Option<Vec<Cow<'a, str>>>,
        #[serde(default)]
        default: Option<Cow<'a, str>>,
    },
    Number {
        #[serde(
            default,
            alias = "min",
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_f64"
        )]
        minimum: Option<f64>,
        #[serde(
            default,
            alias = "max",
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_f64"
        )]
        maximum: Option<f64>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_f64"
        )]
        default: Option<f64>,
    },
    Integer {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_i128"
        )]
        minimum: Option<i128>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_i128"
        )]
        maximum: Option<i128>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_i128"
        )]
        default: Option<i128>,
    },
    Boolean {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_bool"
        )]
        default: Option<bool>,
    },
    Array {
        items: Box<Type<'a>>,
    },
    Object {
        #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
        properties: Option<BTreeMap<Cow<'a, str>, Type<'a>>>,
        #[serde(
            default,
            skip_serializing_if = "IntOrTy::is_unset",
            rename = "additionalProperties"
        )]
        additional_properties: IntOrTy<'a>,
    },
}
