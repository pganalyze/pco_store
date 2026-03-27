use quote::quote;
use syn::Type;

pub fn deserialize_time_range(timestamp_ty: &Type) -> proc_macro2::TokenStream {
    quote! {
        /// Deserializes many different time range formats:
        /// - an array with two strings becomes a normal time range: ["a", "b"] -> a..=b
        /// - an array with one string becomes a single-value time range: ["a"] -> a..=a
        /// - a string literal becomes a single-value time range:           "a" -> a..=a
        fn deserialize_time_range<'de, D>(deserializer: D) -> Result<Option<std::ops::RangeInclusive<#timestamp_ty>>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TimeRange::deserialize(deserializer)?.0)
        }

        #[derive(Debug, PartialEq)]
        struct TimeRange(Option<std::ops::RangeInclusive<#timestamp_ty>>);
        impl<'de> serde::Deserialize<'de> for TimeRange {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_any(TimeRangeVisitor)
            }
        }

        struct TimeRangeVisitor;
        impl<'de> serde::de::Visitor<'de> for TimeRangeVisitor {
            type Value = TimeRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a single time string or an array with 1-2 time strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value.is_empty() {
                    return Ok(TimeRange(None));
                }
                match serde::Deserialize::deserialize(serde::de::value::StrDeserializer::<E>::new(value)) {
                    Ok(start) => Ok(TimeRange(Some(start..=start))),
                    Err(err) => Err(E::custom("invalid time format: ".to_string() + err.to_string().as_str())),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let start = match seq.next_element::<Option<#timestamp_ty>>()? {
                    Some(Some(time)) => time,
                    Some(None) | None => return Ok(TimeRange(None)),
                };
                let end = match seq.next_element::<Option<#timestamp_ty>>()? {
                    Some(Some(time)) => time,
                    Some(None) | None => start,
                };
                Ok(TimeRange(Some(start..=end)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TimeRange(None))
            }
        }
    }
}
