use quote::{ToTokens, quote};
use syn::{ItemStruct, Type};

use super::{Arguments, tokens};

pub fn generate(model: ItemStruct, args: Arguments, using_chrono: bool, timestamp_ty: &Option<Type>) -> proc_macro2::TokenStream {
    let name = model.ident.clone();
    let Arguments { timestamp, group_by, .. } = args;
    let mut filter_fields = Vec::new();
    let mut filter_conditions = Vec::new();
    let mut filter_new_args = Vec::new();
    let mut filter_new_names = Vec::new();
    for field in model.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let ty = field.ty.clone();
        let time = ty.to_token_stream().to_string().contains("Time");
        if time {
            filter_fields.push(quote! {
                #[serde(deserialize_with = "deserialize_time_range")]
                pub #ident: Option<std::ops::RangeInclusive<#ty>>,
            });
            filter_conditions.push(quote! {
                self.#ident.as_ref().map(|t| t.contains(&row.#ident)) != Some(false)
            });
        } else {
            filter_fields.push(quote! {
                #[serde(default)]
                #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
                pub #ident: Vec<#ty>,
            });
            filter_conditions.push(quote! {
                (self.#ident.is_empty() || self.#ident.contains(&row.#ident))
            });
        }
        if group_by.iter().any(|i| *i == ident) || timestamp.as_ref().map(|t| *t == ident).unwrap_or(false) {
            if time {
                filter_new_args.push(quote! { #ident: std::ops::RangeInclusive<#ty>, });
                filter_new_names.push(quote! { #ident: Some(#ident), });
            } else {
                filter_new_args.push(quote! { #ident: &[#ty], });
                filter_new_names.push(quote! { #ident: #ident.into(), });
            }
        }
    }
    let filter_fields = tokens(filter_fields);
    let filter_new_args = tokens(filter_new_args);
    let filter_new_names = tokens(filter_new_names);
    let timestamp_helpers = timestamp.map(|timestamp| {
        let (duration_type, duration_math) = if using_chrono {
            (quote! { chrono::Duration }, quote! { end - start })
        } else {
            (quote! { std::time::Duration }, quote! { end.duration_since(start)? })
        };
        let truncate_nanos = if using_chrono {
            quote! {
                Ok(chrono::DateTime::from_timestamp_micros(time.timestamp_micros()).context("out of range")?)
            }
        } else {
            quote! {
                use std::time::{UNIX_EPOCH, Duration};
                let duration = time.duration_since(UNIX_EPOCH).context("earlier than epoch")?;
                let micros = duration.as_secs() * 1_000_000 + (duration.subsec_nanos() / 1_000) as u64;
                Ok(UNIX_EPOCH + Duration::from_secs(micros / 1_000_000) + Duration::from_micros(micros % 1_000_000))
            }
        };
        quote! {
            /// Convenience function to unwrap the timestamp range lower and upper bounds
            pub fn range_bounds(&self) -> anyhow::Result<(#timestamp_ty, #timestamp_ty)> {
                use anyhow::Context;
                let timestamp = self.#timestamp.clone().context("no timestamp")?;
                Ok((*timestamp.start(), *timestamp.end()))
            }

            /// Convenience function to return the amount of time the filter covers
            pub fn range_duration(&self) -> anyhow::Result<#duration_type> {
                let (start, end) = self.range_bounds()?;
                Ok(#duration_math)
            }

            /// Shifts the filtered time range. This for example makes it easier
            /// to perform two queries: once for "today", and one for "today, 7 days ago".
            /// In that example the second query would do `filter.shift(Duration::days(-7))`
            pub fn range_shift(&mut self, duration: #duration_type) -> anyhow::Result<()> {
                use std::ops::Add;
                let (start, end) = self.range_bounds()?;
                self.#timestamp = Some(start.add(duration)..=end.add(duration));
                Ok(())
            }

            /// Postgres doesn't support nanosecond precision and nor does MacOS, so this
            /// truncates nanosecond precision for timestamp comparisons
            fn range_truncate(&mut self) -> anyhow::Result<()> {
                let (start, end) = self.range_bounds()?;
                self.#timestamp = Some(Self::truncate_nanos(start)?..=Self::truncate_nanos(end)?);
                Ok(())
            }

            fn truncate_nanos(time: #timestamp_ty) -> anyhow::Result<#timestamp_ty> {
                use anyhow::Context;
                #truncate_nanos
            }
        }
    });
    quote! {
        #[serde_with::serde_as]
        #[derive(Debug, Default, serde::Deserialize, Clone, PartialEq)]
        #[serde(deny_unknown_fields)]
        #[doc=concat!(" Generated by pco_store to specify filters when loading [", stringify!(#name), "]")]
        pub struct Filter {
            #filter_fields
        }

        impl Filter {
            /// Builds new filter with the required fields defined by `group_by` and `timestamp`
            pub fn new(#filter_new_args) -> Self {
                Self { #filter_new_names ..Self::default() }
            }

            fn matches(&self, row: &#name) -> bool {
                #(#filter_conditions)&&*
            }

            #timestamp_helpers
        }
    }
}
