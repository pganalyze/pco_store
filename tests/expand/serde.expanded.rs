use serde::Deserialize as _;
pub struct Serde {
    pub id: Uuid,
    pub name: String,
    pub time: DateTime<Utc>,
    pub description: String,
    pub tags: Vec<String>,
    pub nums: Vec<i32>,
    pub map: BTreeMap<String, String>,
    pub json: serde_json::Value,
    pub model: Option<Box<Serde>>,
}
#[automatically_derived]
impl ::core::clone::Clone for Serde {
    #[inline]
    fn clone(&self) -> Serde {
        Serde {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
            time: ::core::clone::Clone::clone(&self.time),
            description: ::core::clone::Clone::clone(&self.description),
            tags: ::core::clone::Clone::clone(&self.tags),
            nums: ::core::clone::Clone::clone(&self.nums),
            map: ::core::clone::Clone::clone(&self.map),
            json: ::core::clone::Clone::clone(&self.json),
            model: ::core::clone::Clone::clone(&self.model),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Serde {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let names: &'static _ = &[
            "id",
            "name",
            "time",
            "description",
            "tags",
            "nums",
            "map",
            "json",
            "model",
        ];
        let values: &[&dyn ::core::fmt::Debug] = &[
            &self.id,
            &self.name,
            &self.time,
            &self.description,
            &self.tags,
            &self.nums,
            &self.map,
            &self.json,
            &&self.model,
        ];
        ::core::fmt::Formatter::debug_struct_fields_finish(f, "Serde", names, values)
    }
}
#[automatically_derived]
impl ::core::default::Default for Serde {
    #[inline]
    fn default() -> Serde {
        Serde {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
            time: ::core::default::Default::default(),
            description: ::core::default::Default::default(),
            tags: ::core::default::Default::default(),
            nums: ::core::default::Default::default(),
            map: ::core::default::Default::default(),
            json: ::core::default::Default::default(),
            model: ::core::default::Default::default(),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Serde {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Serde {
    #[inline]
    fn eq(&self, other: &Serde) -> bool {
        self.id == other.id && self.name == other.name && self.time == other.time
            && self.description == other.description && self.tags == other.tags
            && self.nums == other.nums && self.map == other.map
            && self.json == other.json && self.model == other.model
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl _serde::Serialize for Serde {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> _serde::__private228::Result<__S::Ok, __S::Error>
        where
            __S: _serde::Serializer,
        {
            let mut __serde_state = _serde::Serializer::serialize_struct(
                __serializer,
                "Serde",
                false as usize + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "id",
                &self.id,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "name",
                &self.name,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "time",
                &self.time,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "description",
                &self.description,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "tags",
                &self.tags,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "nums",
                &self.nums,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "map",
                &self.map,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "json",
                &self.json,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "model",
                &self.model,
            )?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Serde {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private228::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __field4,
                __field5,
                __field6,
                __field7,
                __field8,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private228::Ok(__Field::__field0),
                        1u64 => _serde::__private228::Ok(__Field::__field1),
                        2u64 => _serde::__private228::Ok(__Field::__field2),
                        3u64 => _serde::__private228::Ok(__Field::__field3),
                        4u64 => _serde::__private228::Ok(__Field::__field4),
                        5u64 => _serde::__private228::Ok(__Field::__field5),
                        6u64 => _serde::__private228::Ok(__Field::__field6),
                        7u64 => _serde::__private228::Ok(__Field::__field7),
                        8u64 => _serde::__private228::Ok(__Field::__field8),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "id" => _serde::__private228::Ok(__Field::__field0),
                        "name" => _serde::__private228::Ok(__Field::__field1),
                        "time" => _serde::__private228::Ok(__Field::__field2),
                        "description" => _serde::__private228::Ok(__Field::__field3),
                        "tags" => _serde::__private228::Ok(__Field::__field4),
                        "nums" => _serde::__private228::Ok(__Field::__field5),
                        "map" => _serde::__private228::Ok(__Field::__field6),
                        "json" => _serde::__private228::Ok(__Field::__field7),
                        "model" => _serde::__private228::Ok(__Field::__field8),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"id" => _serde::__private228::Ok(__Field::__field0),
                        b"name" => _serde::__private228::Ok(__Field::__field1),
                        b"time" => _serde::__private228::Ok(__Field::__field2),
                        b"description" => _serde::__private228::Ok(__Field::__field3),
                        b"tags" => _serde::__private228::Ok(__Field::__field4),
                        b"nums" => _serde::__private228::Ok(__Field::__field5),
                        b"map" => _serde::__private228::Ok(__Field::__field6),
                        b"json" => _serde::__private228::Ok(__Field::__field7),
                        b"model" => _serde::__private228::Ok(__Field::__field8),
                        _ => _serde::__private228::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private228::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private228::PhantomData<Serde>,
                lifetime: _serde::__private228::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Serde;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "struct Serde",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match _serde::de::SeqAccess::next_element::<
                        Uuid,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field1 = match _serde::de::SeqAccess::next_element::<
                        String,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field2 = match _serde::de::SeqAccess::next_element::<
                        DateTime<Utc>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field3 = match _serde::de::SeqAccess::next_element::<
                        String,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    3usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field4 = match _serde::de::SeqAccess::next_element::<
                        Vec<String>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    4usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field5 = match _serde::de::SeqAccess::next_element::<
                        Vec<i32>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    5usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field6 = match _serde::de::SeqAccess::next_element::<
                        BTreeMap<String, String>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    6usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field7 = match _serde::de::SeqAccess::next_element::<
                        serde_json::Value,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    7usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field8 = match _serde::de::SeqAccess::next_element::<
                        Option<Box<Serde>>,
                    >(&mut __seq)? {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    8usize,
                                    &"struct Serde with 9 elements",
                                ),
                            );
                        }
                    };
                    _serde::__private228::Ok(Serde {
                        id: __field0,
                        name: __field1,
                        time: __field2,
                        description: __field3,
                        tags: __field4,
                        nums: __field5,
                        map: __field6,
                        json: __field7,
                        model: __field8,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private228::Option<Uuid> = _serde::__private228::None;
                    let mut __field1: _serde::__private228::Option<String> = _serde::__private228::None;
                    let mut __field2: _serde::__private228::Option<DateTime<Utc>> = _serde::__private228::None;
                    let mut __field3: _serde::__private228::Option<String> = _serde::__private228::None;
                    let mut __field4: _serde::__private228::Option<Vec<String>> = _serde::__private228::None;
                    let mut __field5: _serde::__private228::Option<Vec<i32>> = _serde::__private228::None;
                    let mut __field6: _serde::__private228::Option<
                        BTreeMap<String, String>,
                    > = _serde::__private228::None;
                    let mut __field7: _serde::__private228::Option<serde_json::Value> = _serde::__private228::None;
                    let mut __field8: _serde::__private228::Option<Option<Box<Serde>>> = _serde::__private228::None;
                    while let _serde::__private228::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private228::Option::is_some(&__field0) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("id"),
                                    );
                                }
                                __field0 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<Uuid>(&mut __map)?,
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private228::Option::is_some(&__field1) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("name"),
                                    );
                                }
                                __field1 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                );
                            }
                            __Field::__field2 => {
                                if _serde::__private228::Option::is_some(&__field2) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("time"),
                                    );
                                }
                                __field2 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        DateTime<Utc>,
                                    >(&mut __map)?,
                                );
                            }
                            __Field::__field3 => {
                                if _serde::__private228::Option::is_some(&__field3) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "description",
                                        ),
                                    );
                                }
                                __field3 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                );
                            }
                            __Field::__field4 => {
                                if _serde::__private228::Option::is_some(&__field4) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("tags"),
                                    );
                                }
                                __field4 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        Vec<String>,
                                    >(&mut __map)?,
                                );
                            }
                            __Field::__field5 => {
                                if _serde::__private228::Option::is_some(&__field5) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("nums"),
                                    );
                                }
                                __field5 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<Vec<i32>>(&mut __map)?,
                                );
                            }
                            __Field::__field6 => {
                                if _serde::__private228::Option::is_some(&__field6) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("map"),
                                    );
                                }
                                __field6 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        BTreeMap<String, String>,
                                    >(&mut __map)?,
                                );
                            }
                            __Field::__field7 => {
                                if _serde::__private228::Option::is_some(&__field7) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("json"),
                                    );
                                }
                                __field7 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        serde_json::Value,
                                    >(&mut __map)?,
                                );
                            }
                            __Field::__field8 => {
                                if _serde::__private228::Option::is_some(&__field8) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("model"),
                                    );
                                }
                                __field8 = _serde::__private228::Some(
                                    _serde::de::MapAccess::next_value::<
                                        Option<Box<Serde>>,
                                    >(&mut __map)?,
                                );
                            }
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private228::Some(__field0) => __field0,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("id")?
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private228::Some(__field1) => __field1,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("name")?
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private228::Some(__field2) => __field2,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("time")?
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::__private228::Some(__field3) => __field3,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("description")?
                        }
                    };
                    let __field4 = match __field4 {
                        _serde::__private228::Some(__field4) => __field4,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("tags")?
                        }
                    };
                    let __field5 = match __field5 {
                        _serde::__private228::Some(__field5) => __field5,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("nums")?
                        }
                    };
                    let __field6 = match __field6 {
                        _serde::__private228::Some(__field6) => __field6,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("map")?
                        }
                    };
                    let __field7 = match __field7 {
                        _serde::__private228::Some(__field7) => __field7,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("json")?
                        }
                    };
                    let __field8 = match __field8 {
                        _serde::__private228::Some(__field8) => __field8,
                        _serde::__private228::None => {
                            _serde::__private228::de::missing_field("model")?
                        }
                    };
                    _serde::__private228::Ok(Serde {
                        id: __field0,
                        name: __field1,
                        time: __field2,
                        description: __field3,
                        tags: __field4,
                        nums: __field5,
                        map: __field6,
                        json: __field7,
                        model: __field8,
                    })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &[
                "id",
                "name",
                "time",
                "description",
                "tags",
                "nums",
                "map",
                "json",
                "model",
            ];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "Serde",
                FIELDS,
                __Visitor {
                    marker: _serde::__private228::PhantomData::<Serde>,
                    lifetime: _serde::__private228::PhantomData,
                },
            )
        }
    }
};
/// Generated by pco_store to store and load compressed versions of [Serde]
pub struct CompressedSerdes {
    filter: Option<Filter>,
    id: Uuid,
    name: String,
    time: Vec<u8>,
    description: Vec<u8>,
    tags: Vec<u8>,
    nums: Vec<u8>,
    map: Vec<u8>,
    json: Vec<u8>,
    model: Vec<u8>,
}
impl CompressedSerdes {
    /// Loads data for the specified filters.
    pub async fn load(
        db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        mut filter: Filter,
        fields: impl TryInto<Fields>,
    ) -> anyhow::Result<Vec<CompressedSerdes>> {
        let mut fields = fields
            .try_into()
            .map_err(|_| anyhow::Error::msg("unknown field"))?;
        fields.merge_filter(&filter);
        if filter.id.is_empty() {
            return Err(anyhow::Error::msg("id".to_string() + " is required"));
        }
        if filter.name.is_empty() {
            return Err(anyhow::Error::msg("name".to_string() + " is required"));
        }
        if filter.time.is_none() {
            return Err(anyhow::Error::msg("time".to_string() + " is required"));
        }
        filter.range_truncate()?;
        let sql = "SELECT ".to_string() + fields.select().as_str() + " FROM " + "serdes"
            + " WHERE "
            + "id = ANY($1) AND name = ANY($2) AND end_at >= $3 AND start_at <= $4";
        let mut results = Vec::new();
        for row in db
            .query(
                &db.prepare_cached(&sql).await?,
                &[
                    &filter.id,
                    &filter.name,
                    filter.time.as_ref().unwrap().start(),
                    filter.time.as_ref().unwrap().end(),
                ],
            )
            .await?
        {
            results.push(fields.load_from_row(row, Some(filter.clone()))?);
        }
        Ok(results)
    }
    /// Deletes data for the specified filters, returning it to the caller.
    ///
    /// Note that all rows are returned from [decompress][Self::decompress] even if post-decompress filters would normally apply.
    pub async fn delete(
        db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        mut filter: Filter,
        fields: impl TryInto<Fields>,
    ) -> anyhow::Result<Vec<CompressedSerdes>> {
        let mut fields = fields
            .try_into()
            .map_err(|_| anyhow::Error::msg("unknown field"))?;
        fields.merge_filter(&filter);
        if filter.id.is_empty() {
            return Err(anyhow::Error::msg("id".to_string() + " is required"));
        }
        if filter.name.is_empty() {
            return Err(anyhow::Error::msg("name".to_string() + " is required"));
        }
        if filter.time.is_none() {
            return Err(anyhow::Error::msg("time".to_string() + " is required"));
        }
        filter.range_truncate()?;
        let sql = "DELETE FROM ".to_string() + "serdes" + " WHERE "
            + "id = ANY($1) AND name = ANY($2) AND end_at >= $3 AND start_at <= $4"
            + " RETURNING " + fields.select().as_str();
        let mut results = Vec::new();
        for row in db
            .query(
                &db.prepare_cached(&sql).await?,
                &[
                    &filter.id,
                    &filter.name,
                    filter.time.as_ref().unwrap().start(),
                    filter.time.as_ref().unwrap().end(),
                ],
            )
            .await?
        {
            results.push(fields.load_from_row(row, None)?);
        }
        Ok(results)
    }
    /// Decompresses a group of data points.
    pub fn decompress(self) -> anyhow::Result<Vec<Serde>> {
        let mut results = Vec::new();
        let time: Vec<u64> = if self.time.is_empty() {
            Vec::new()
        } else {
            ::pco::standalone::simple_decompress(&self.time)?
        };
        let mut description = serde_decompress::<String>(&self.description);
        let mut tags = serde_decompress::<Vec<String>>(&self.tags);
        let mut nums: std::vec::IntoIter<Vec<i32>> = pco_decompress_nested(self.nums)?
            .into_iter();
        let mut map = serde_decompress::<BTreeMap<String, String>>(&self.map);
        let mut json = serde_decompress::<serde_json::Value>(&self.json);
        let mut model = serde_decompress::<Option<Box<Serde>>>(&self.model);
        let len = [time.len()].into_iter().max().unwrap_or(0);
        for index in 0..len {
            let row = Serde {
                id: self.id.clone(),
                name: self.name.clone(),
                time: chrono::DateTime::from_timestamp_micros(
                        time.get(index).cloned().unwrap_or_default() as i64,
                    )
                    .unwrap(),
                description: description.next().transpose()?.unwrap_or_default(),
                tags: tags.next().transpose()?.unwrap_or_default(),
                nums: nums.next().unwrap_or_default(),
                map: map.next().transpose()?.unwrap_or_default(),
                json: json.next().transpose()?.unwrap_or_default(),
                model: model.next().transpose()?.unwrap_or_default(),
            };
            if self.filter.as_ref().map(|f| f.matches(&row)) != Some(false) {
                results.push(row);
            }
        }
        Ok(results)
    }
    /// Writes the data to disk.
    pub async fn store(
        db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        rows: Vec<Serde>,
    ) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut grouped_rows: ahash::AHashMap<_, Vec<Serde>> = ahash::AHashMap::new();
        for row in rows {
            grouped_rows
                .entry((row.id.clone(), row.name.clone()))
                .or_default()
                .push(row);
        }
        let sql = "COPY serdes (id, name, start_at, end_at, time, description, tags, nums, map, json, model) FROM STDIN BINARY";
        let types = &[
            tokio_postgres::types::Type::UUID,
            tokio_postgres::types::Type::TEXT,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
        ];
        let stmt = db.copy_in(&db.prepare_cached(&sql).await?).await?;
        let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(stmt, types);
        let mut writer = writer;
        #[allow(unused_mut)]
        let mut writer = unsafe {
            ::pin_utils::core_reexport::pin::Pin::new_unchecked(&mut writer)
        };
        for rows in grouped_rows.into_values() {
            let time: Vec<_> = rows.iter().map(|s| s.time).collect();
            let start_at = *time.iter().min().unwrap();
            let end_at = *time.iter().max().unwrap();
            let time: Vec<u64> = time
                .into_iter()
                .map(|t| t.timestamp_micros() as u64)
                .collect();
            writer
                .as_mut()
                .write(
                    &[
                        &rows[0].id,
                        &rows[0].name,
                        &start_at,
                        &end_at,
                        &::pco::standalone::simple_compress(
                                &time,
                                &::pco::ChunkConfig::default(),
                            )
                            .unwrap(),
                        &serde_compress(
                            rows
                                .iter()
                                .map(|r| r.description.clone())
                                .collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.tags.clone()).collect::<Vec<_>>(),
                        )?,
                        &pco_compress_nested(
                            rows
                                .iter()
                                .map(|r| r.nums.iter().map(|v| *v).collect::<Vec<_>>())
                                .collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.map.clone()).collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.json.clone()).collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.model.clone()).collect::<Vec<_>>(),
                        )?,
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
    /// Writes the data to disk, with the provided grouping closure applied.
    ///
    /// This can be used to improve the compression ratio and reduce read IO, for example
    /// by compacting real-time data into a single row per hour / day / week.
    pub async fn store_grouped<F, R>(
        db: &impl ::std::ops::Deref<Target = deadpool_postgres::ClientWrapper>,
        rows: Vec<Serde>,
        grouping: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(&Serde) -> R,
        R: Eq + std::hash::Hash,
    {
        if rows.is_empty() {
            return Ok(());
        }
        let mut grouped_rows: ahash::AHashMap<_, Vec<Serde>> = ahash::AHashMap::new();
        for row in rows {
            grouped_rows
                .entry((row.id.clone(), row.name.clone(), grouping(&row)))
                .or_default()
                .push(row);
        }
        let sql = "COPY serdes (id, name, start_at, end_at, time, description, tags, nums, map, json, model) FROM STDIN BINARY";
        let types = &[
            tokio_postgres::types::Type::UUID,
            tokio_postgres::types::Type::TEXT,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::TIMESTAMPTZ,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
            tokio_postgres::types::Type::BYTEA,
        ];
        let stmt = db.copy_in(&db.prepare_cached(&sql).await?).await?;
        let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(stmt, types);
        let mut writer = writer;
        #[allow(unused_mut)]
        let mut writer = unsafe {
            ::pin_utils::core_reexport::pin::Pin::new_unchecked(&mut writer)
        };
        for rows in grouped_rows.into_values() {
            let time: Vec<_> = rows.iter().map(|s| s.time).collect();
            let start_at = *time.iter().min().unwrap();
            let end_at = *time.iter().max().unwrap();
            let time: Vec<u64> = time
                .into_iter()
                .map(|t| t.timestamp_micros() as u64)
                .collect();
            writer
                .as_mut()
                .write(
                    &[
                        &rows[0].id,
                        &rows[0].name,
                        &start_at,
                        &end_at,
                        &::pco::standalone::simple_compress(
                                &time,
                                &::pco::ChunkConfig::default(),
                            )
                            .unwrap(),
                        &serde_compress(
                            rows
                                .iter()
                                .map(|r| r.description.clone())
                                .collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.tags.clone()).collect::<Vec<_>>(),
                        )?,
                        &pco_compress_nested(
                            rows
                                .iter()
                                .map(|r| r.nums.iter().map(|v| *v).collect::<Vec<_>>())
                                .collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.map.clone()).collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.json.clone()).collect::<Vec<_>>(),
                        )?,
                        &serde_compress(
                            rows.iter().map(|r| r.model.clone()).collect::<Vec<_>>(),
                        )?,
                    ],
                )
                .await?;
        }
        writer.finish().await?;
        Ok(())
    }
}
#[serde(deny_unknown_fields)]
/// Generated by pco_store to specify filters when loading [Serde]
pub struct Filter {
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub id: Vec<Uuid>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub name: Vec<String>,
    #[serde(deserialize_with = "deserialize_time_range")]
    pub time: Option<std::ops::RangeInclusive<DateTime<Utc>>>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub description: Vec<String>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub tags: Vec<Vec<String>>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub nums: Vec<Vec<i32>>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub map: Vec<BTreeMap<String, String>>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub json: Vec<serde_json::Value>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull<serde_with::OneOrMany<_>>")]
    #[serde(
        deserialize_with = ":: serde_with :: As :: < serde_with :: DefaultOnNull < serde_with :: OneOrMany\n< :: serde_with :: Same > > > :: deserialize"
    )]
    pub model: Vec<Option<Box<Serde>>>,
}
#[automatically_derived]
impl ::core::fmt::Debug for Filter {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let names: &'static _ = &[
            "id",
            "name",
            "time",
            "description",
            "tags",
            "nums",
            "map",
            "json",
            "model",
        ];
        let values: &[&dyn ::core::fmt::Debug] = &[
            &self.id,
            &self.name,
            &self.time,
            &self.description,
            &self.tags,
            &self.nums,
            &self.map,
            &self.json,
            &&self.model,
        ];
        ::core::fmt::Formatter::debug_struct_fields_finish(f, "Filter", names, values)
    }
}
#[automatically_derived]
impl ::core::default::Default for Filter {
    #[inline]
    fn default() -> Filter {
        Filter {
            id: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
            time: ::core::default::Default::default(),
            description: ::core::default::Default::default(),
            tags: ::core::default::Default::default(),
            nums: ::core::default::Default::default(),
            map: ::core::default::Default::default(),
            json: ::core::default::Default::default(),
            model: ::core::default::Default::default(),
        }
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de> _serde::Deserialize<'de> for Filter {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private228::Result<Self, __D::Error>
        where
            __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __field4,
                __field5,
                __field6,
                __field7,
                __field8,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private228::Ok(__Field::__field0),
                        1u64 => _serde::__private228::Ok(__Field::__field1),
                        2u64 => _serde::__private228::Ok(__Field::__field2),
                        3u64 => _serde::__private228::Ok(__Field::__field3),
                        4u64 => _serde::__private228::Ok(__Field::__field4),
                        5u64 => _serde::__private228::Ok(__Field::__field5),
                        6u64 => _serde::__private228::Ok(__Field::__field6),
                        7u64 => _serde::__private228::Ok(__Field::__field7),
                        8u64 => _serde::__private228::Ok(__Field::__field8),
                        _ => {
                            _serde::__private228::Err(
                                _serde::de::Error::invalid_value(
                                    _serde::de::Unexpected::Unsigned(__value),
                                    &"field index 0 <= i < 9",
                                ),
                            )
                        }
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "id" => _serde::__private228::Ok(__Field::__field0),
                        "name" => _serde::__private228::Ok(__Field::__field1),
                        "time" => _serde::__private228::Ok(__Field::__field2),
                        "description" => _serde::__private228::Ok(__Field::__field3),
                        "tags" => _serde::__private228::Ok(__Field::__field4),
                        "nums" => _serde::__private228::Ok(__Field::__field5),
                        "map" => _serde::__private228::Ok(__Field::__field6),
                        "json" => _serde::__private228::Ok(__Field::__field7),
                        "model" => _serde::__private228::Ok(__Field::__field8),
                        _ => {
                            _serde::__private228::Err(
                                _serde::de::Error::unknown_field(__value, FIELDS),
                            )
                        }
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private228::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"id" => _serde::__private228::Ok(__Field::__field0),
                        b"name" => _serde::__private228::Ok(__Field::__field1),
                        b"time" => _serde::__private228::Ok(__Field::__field2),
                        b"description" => _serde::__private228::Ok(__Field::__field3),
                        b"tags" => _serde::__private228::Ok(__Field::__field4),
                        b"nums" => _serde::__private228::Ok(__Field::__field5),
                        b"map" => _serde::__private228::Ok(__Field::__field6),
                        b"json" => _serde::__private228::Ok(__Field::__field7),
                        b"model" => _serde::__private228::Ok(__Field::__field8),
                        _ => {
                            let __value = &_serde::__private228::from_utf8_lossy(
                                __value,
                            );
                            _serde::__private228::Err(
                                _serde::de::Error::unknown_field(__value, FIELDS),
                            )
                        }
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private228::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private228::PhantomData<Filter>,
                lifetime: _serde::__private228::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = Filter;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private228::Formatter,
                ) -> _serde::__private228::fmt::Result {
                    _serde::__private228::Formatter::write_str(
                        __formatter,
                        "struct Filter",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<Uuid>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field1 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<String>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field2 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Option<std::ops::RangeInclusive<DateTime<Utc>>>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: deserialize_time_range(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                _serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct Filter with 9 elements",
                                ),
                            );
                        }
                    };
                    let __field3 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<String>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field4 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<Vec<String>>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field5 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<Vec<i32>>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field6 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<BTreeMap<String, String>>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field7 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<serde_json::Value>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field8 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de> {
                            value: Vec<Option<Box<Serde>>>,
                            phantom: _serde::__private228::PhantomData<Filter>,
                            lifetime: _serde::__private228::PhantomData<&'de ()>,
                        }
                        #[automatically_derived]
                        impl<'de> _serde::Deserialize<'de> for __DeserializeWith<'de> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private228::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private228::Ok(__DeserializeWith {
                                    value: ::serde_with::As::<
                                        serde_with::DefaultOnNull<
                                            serde_with::OneOrMany<::serde_with::Same>,
                                        >,
                                    >::deserialize(__deserializer)?,
                                    phantom: _serde::__private228::PhantomData,
                                    lifetime: _serde::__private228::PhantomData,
                                })
                            }
                        }
                        _serde::__private228::Option::map(
                            _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de>,
                            >(&mut __seq)?,
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private228::Some(__value) => __value,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    _serde::__private228::Ok(Filter {
                        id: __field0,
                        name: __field1,
                        time: __field2,
                        description: __field3,
                        tags: __field4,
                        nums: __field5,
                        map: __field6,
                        json: __field7,
                        model: __field8,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private228::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private228::Option<Vec<Uuid>> = _serde::__private228::None;
                    let mut __field1: _serde::__private228::Option<Vec<String>> = _serde::__private228::None;
                    let mut __field2: _serde::__private228::Option<
                        Option<std::ops::RangeInclusive<DateTime<Utc>>>,
                    > = _serde::__private228::None;
                    let mut __field3: _serde::__private228::Option<Vec<String>> = _serde::__private228::None;
                    let mut __field4: _serde::__private228::Option<Vec<Vec<String>>> = _serde::__private228::None;
                    let mut __field5: _serde::__private228::Option<Vec<Vec<i32>>> = _serde::__private228::None;
                    let mut __field6: _serde::__private228::Option<
                        Vec<BTreeMap<String, String>>,
                    > = _serde::__private228::None;
                    let mut __field7: _serde::__private228::Option<
                        Vec<serde_json::Value>,
                    > = _serde::__private228::None;
                    let mut __field8: _serde::__private228::Option<
                        Vec<Option<Box<Serde>>>,
                    > = _serde::__private228::None;
                    while let _serde::__private228::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private228::Option::is_some(&__field0) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("id"),
                                    );
                                }
                                __field0 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<Uuid>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field1 => {
                                if _serde::__private228::Option::is_some(&__field1) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("name"),
                                    );
                                }
                                __field1 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<String>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field2 => {
                                if _serde::__private228::Option::is_some(&__field2) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("time"),
                                    );
                                }
                                __field2 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Option<std::ops::RangeInclusive<DateTime<Utc>>>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: deserialize_time_range(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field3 => {
                                if _serde::__private228::Option::is_some(&__field3) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "description",
                                        ),
                                    );
                                }
                                __field3 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<String>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field4 => {
                                if _serde::__private228::Option::is_some(&__field4) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("tags"),
                                    );
                                }
                                __field4 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<Vec<String>>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field5 => {
                                if _serde::__private228::Option::is_some(&__field5) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("nums"),
                                    );
                                }
                                __field5 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<Vec<i32>>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field6 => {
                                if _serde::__private228::Option::is_some(&__field6) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("map"),
                                    );
                                }
                                __field6 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<BTreeMap<String, String>>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field7 => {
                                if _serde::__private228::Option::is_some(&__field7) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("json"),
                                    );
                                }
                                __field7 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<serde_json::Value>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field8 => {
                                if _serde::__private228::Option::is_some(&__field8) {
                                    return _serde::__private228::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("model"),
                                    );
                                }
                                __field8 = _serde::__private228::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de> {
                                        value: Vec<Option<Box<Serde>>>,
                                        phantom: _serde::__private228::PhantomData<Filter>,
                                        lifetime: _serde::__private228::PhantomData<&'de ()>,
                                    }
                                    #[automatically_derived]
                                    impl<'de> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private228::Result<Self, __D::Error>
                                        where
                                            __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private228::Ok(__DeserializeWith {
                                                value: ::serde_with::As::<
                                                    serde_with::DefaultOnNull<
                                                        serde_with::OneOrMany<::serde_with::Same>,
                                                    >,
                                                >::deserialize(__deserializer)?,
                                                phantom: _serde::__private228::PhantomData,
                                                lifetime: _serde::__private228::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de>,
                                    >(&mut __map) {
                                        _serde::__private228::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private228::Err(__err) => {
                                            return _serde::__private228::Err(__err);
                                        }
                                    }
                                });
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private228::Some(__field0) => __field0,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private228::Some(__field1) => __field1,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private228::Some(__field2) => __field2,
                        _serde::__private228::None => {
                            return _serde::__private228::Err(
                                <__A::Error as _serde::de::Error>::missing_field("time"),
                            );
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::__private228::Some(__field3) => __field3,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field4 = match __field4 {
                        _serde::__private228::Some(__field4) => __field4,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field5 = match __field5 {
                        _serde::__private228::Some(__field5) => __field5,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field6 = match __field6 {
                        _serde::__private228::Some(__field6) => __field6,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field7 = match __field7 {
                        _serde::__private228::Some(__field7) => __field7,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    let __field8 = match __field8 {
                        _serde::__private228::Some(__field8) => __field8,
                        _serde::__private228::None => {
                            _serde::__private228::Default::default()
                        }
                    };
                    _serde::__private228::Ok(Filter {
                        id: __field0,
                        name: __field1,
                        time: __field2,
                        description: __field3,
                        tags: __field4,
                        nums: __field5,
                        map: __field6,
                        json: __field7,
                        model: __field8,
                    })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &[
                "id",
                "name",
                "time",
                "description",
                "tags",
                "nums",
                "map",
                "json",
                "model",
            ];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "Filter",
                FIELDS,
                __Visitor {
                    marker: _serde::__private228::PhantomData::<Filter>,
                    lifetime: _serde::__private228::PhantomData,
                },
            )
        }
    }
};
#[automatically_derived]
impl ::core::clone::Clone for Filter {
    #[inline]
    fn clone(&self) -> Filter {
        Filter {
            id: ::core::clone::Clone::clone(&self.id),
            name: ::core::clone::Clone::clone(&self.name),
            time: ::core::clone::Clone::clone(&self.time),
            description: ::core::clone::Clone::clone(&self.description),
            tags: ::core::clone::Clone::clone(&self.tags),
            nums: ::core::clone::Clone::clone(&self.nums),
            map: ::core::clone::Clone::clone(&self.map),
            json: ::core::clone::Clone::clone(&self.json),
            model: ::core::clone::Clone::clone(&self.model),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Filter {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Filter {
    #[inline]
    fn eq(&self, other: &Filter) -> bool {
        self.id == other.id && self.name == other.name && self.time == other.time
            && self.description == other.description && self.tags == other.tags
            && self.nums == other.nums && self.map == other.map
            && self.json == other.json && self.model == other.model
    }
}
impl Filter {
    /// Builds new filter with the required fields defined by `group_by` and `timestamp`
    pub fn new(
        id: &[Uuid],
        name: &[String],
        time: std::ops::RangeInclusive<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            time: Some(time),
            ..Self::default()
        }
    }
    fn matches(&self, row: &Serde) -> bool {
        (self.id.is_empty() || self.id.contains(&row.id))
            && (self.name.is_empty() || self.name.contains(&row.name))
            && self.time.as_ref().map(|t| t.contains(&row.time)) != Some(false)
            && (self.description.is_empty()
                || self.description.contains(&row.description))
            && (self.tags.is_empty() || self.tags.contains(&row.tags))
            && (self.nums.is_empty() || self.nums.contains(&row.nums))
            && (self.map.is_empty() || self.map.contains(&row.map))
            && (self.json.is_empty() || self.json.contains(&row.json))
            && (self.model.is_empty() || self.model.contains(&row.model))
    }
    /// Convenience function to unwrap the timestamp range lower and upper bounds
    pub fn range_bounds(&self) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
        use anyhow::Context;
        let timestamp = self.time.clone().context("no timestamp")?;
        Ok((*timestamp.start(), *timestamp.end()))
    }
    /// Convenience function to return the amount of time the filter covers
    pub fn range_duration(&self) -> anyhow::Result<chrono::Duration> {
        let (start, end) = self.range_bounds()?;
        Ok(end - start)
    }
    /// Shifts the filtered time range. This for example makes it easier
    /// to perform two queries: once for "today", and one for "today, 7 days ago".
    /// In that example the second query would do `filter.shift(Duration::days(-7))`
    pub fn range_shift(&mut self, duration: chrono::Duration) -> anyhow::Result<()> {
        use std::ops::Add;
        let (start, end) = self.range_bounds()?;
        self.time = Some(start.add(duration)..=end.add(duration));
        Ok(())
    }
    /// Postgres doesn't support nanosecond precision and nor does MacOS, so this
    /// truncates nanosecond precision for timestamp comparisons
    fn range_truncate(&mut self) -> anyhow::Result<()> {
        let (start, end) = self.range_bounds()?;
        self.time = Some(Self::truncate_nanos(start)?..=Self::truncate_nanos(end)?);
        Ok(())
    }
    fn truncate_nanos(time: DateTime<Utc>) -> anyhow::Result<DateTime<Utc>> {
        use anyhow::Context;
        Ok(
            chrono::DateTime::from_timestamp_micros(time.timestamp_micros())
                .context("out of range")?,
        )
    }
}
/// Generated by pco_store to choose which fields to decompress when loading [Serde]
pub struct Fields {
    id: bool,
    name: bool,
    time: bool,
    description: bool,
    tags: bool,
    nums: bool,
    map: bool,
    json: bool,
    model: bool,
}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Fields {}
#[automatically_derived]
impl ::core::clone::Clone for Fields {
    #[inline]
    fn clone(&self) -> Fields {
        let _: ::core::clone::AssertParamIsClone<bool>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for Fields {}
#[automatically_derived]
impl ::core::fmt::Debug for Fields {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let names: &'static _ = &[
            "id",
            "name",
            "time",
            "description",
            "tags",
            "nums",
            "map",
            "json",
            "model",
        ];
        let values: &[&dyn ::core::fmt::Debug] = &[
            &self.id,
            &self.name,
            &self.time,
            &self.description,
            &self.tags,
            &self.nums,
            &self.map,
            &self.json,
            &&self.model,
        ];
        ::core::fmt::Formatter::debug_struct_fields_finish(f, "Fields", names, values)
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Fields {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Fields {
    #[inline]
    fn eq(&self, other: &Fields) -> bool {
        self.id == other.id && self.name == other.name && self.time == other.time
            && self.description == other.description && self.tags == other.tags
            && self.nums == other.nums && self.map == other.map
            && self.json == other.json && self.model == other.model
    }
}
impl Fields {
    pub fn new(fields: &[&str]) -> anyhow::Result<Self> {
        fields.try_into().map_err(|e| anyhow::Error::msg(e))
    }
    pub fn required() -> Self {
        Self {
            id: true,
            name: true,
            time: true,
            description: false,
            tags: false,
            nums: false,
            map: false,
            json: false,
            model: false,
        }
    }
    fn merge_filter(&mut self, filter: &Filter) {
        (!filter.description.is_empty()).then(|| self.description = true);
        (!filter.tags.is_empty()).then(|| self.tags = true);
        (!filter.nums.is_empty()).then(|| self.nums = true);
        (!filter.map.is_empty()).then(|| self.map = true);
        (!filter.json.is_empty()).then(|| self.json = true);
        (!filter.model.is_empty()).then(|| self.model = true);
    }
    fn select(&self) -> String {
        let mut fields = Vec::new();
        self.id.then(|| fields.push("id"));
        self.name.then(|| fields.push("name"));
        self.time.then(|| fields.push("time"));
        self.description.then(|| fields.push("description"));
        self.tags.then(|| fields.push("tags"));
        self.nums.then(|| fields.push("nums"));
        self.map.then(|| fields.push("map"));
        self.json.then(|| fields.push("json"));
        self.model.then(|| fields.push("model"));
        fields.join(", ")
    }
    fn load_from_row(
        &self,
        row: tokio_postgres::Row,
        filter: Option<Filter>,
    ) -> anyhow::Result<CompressedSerdes> {
        let mut index = 0;
        Ok(CompressedSerdes {
            filter,
            id: if self.id {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            name: if self.name {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            time: if self.time {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            description: if self.description {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            tags: if self.tags {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            nums: if self.nums {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            map: if self.map {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            json: if self.json {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
            model: if self.model {
                let v = row.get(index);
                index += 1;
                v
            } else {
                Default::default()
            },
        })
    }
}
impl Default for Fields {
    fn default() -> Self {
        Self {
            id: true,
            name: true,
            time: true,
            description: true,
            tags: true,
            nums: true,
            map: true,
            json: true,
            model: true,
        }
    }
}
impl TryFrom<&[&str]> for Fields {
    type Error = &'static str;
    fn try_from(input: &[&str]) -> Result<Self, Self::Error> {
        let mut fields = Self::required();
        for s in input {
            match *s {
                "id" => fields.id = true,
                "name" => fields.name = true,
                "time" => fields.time = true,
                "description" => fields.description = true,
                "tags" => fields.tags = true,
                "nums" => fields.nums = true,
                "map" => fields.map = true,
                "json" => fields.json = true,
                "model" => fields.model = true,
                _ => return Err("unknown field"),
            }
        }
        Ok(fields)
    }
}
impl<const N: usize> TryFrom<&[&str; N]> for Fields {
    type Error = &'static str;
    fn try_from(input: &[&str; N]) -> Result<Self, Self::Error> {
        Self::try_from(&input[..])
    }
}
impl TryFrom<Vec<String>> for Fields {
    type Error = &'static str;
    fn try_from(input: Vec<String>) -> Result<Self, Self::Error> {
        let input: Vec<_> = input.iter().map(|s| s.as_str()).collect();
        Self::try_from(input.as_slice())
    }
}
impl From<()> for Fields {
    fn from(_: ()) -> Self {
        Self::default()
    }
}
impl<'de> serde::Deserialize<'de> for Fields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FieldsVisitor)
    }
}
struct FieldsVisitor;
impl<'de> serde::de::Visitor<'de> for FieldsVisitor {
    type Value = Fields;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of strings matching the struct fields")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut fields = Vec::new();
        while let Some(field) = seq.next_element()? {
            fields.push(field);
        }
        Fields::try_from(fields).map_err(serde::de::Error::custom)
    }
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Fields::default())
    }
}
/// Deserializes many different time range formats:
/// - an array with two strings becomes a normal time range: ["a", "b"] -> a..=b
/// - an array with one string becomes a single-value time range: ["a"] -> a..=a
/// - a string literal becomes a single-value time range:           "a" -> a..=a
fn deserialize_time_range<'de, D>(
    deserializer: D,
) -> Result<Option<std::ops::RangeInclusive<DateTime<Utc>>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(TimeRange::deserialize(deserializer)?.0)
}
struct TimeRange(Option<std::ops::RangeInclusive<DateTime<Utc>>>);
#[automatically_derived]
impl ::core::fmt::Debug for TimeRange {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TimeRange", &&self.0)
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for TimeRange {}
#[automatically_derived]
impl ::core::cmp::PartialEq for TimeRange {
    #[inline]
    fn eq(&self, other: &TimeRange) -> bool {
        self.0 == other.0
    }
}
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
        match serde::Deserialize::deserialize(
            serde::de::value::StrDeserializer::<E>::new(value),
        ) {
            Ok(start) => Ok(TimeRange(Some(start..=start))),
            Err(err) => {
                Err(
                    E::custom(
                        "invalid time format: ".to_string() + err.to_string().as_str(),
                    ),
                )
            }
        }
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let start = match seq.next_element::<Option<DateTime<Utc>>>()? {
            Some(Some(time)) => time,
            Some(None) | None => return Ok(TimeRange(None)),
        };
        let end = match seq.next_element::<Option<DateTime<Utc>>>()? {
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
fn serde_compress<T>(items: Vec<T>) -> anyhow::Result<Vec<u8>>
where
    T: serde::Serialize,
{
    use std::io::Write;
    let mut output = Vec::new();
    let mut encoder = zstd::stream::write::Encoder::new(&mut output, 3)?;
    for item in items {
        rmp_serde::encode::write(&mut encoder, &item)?;
    }
    encoder.finish()?;
    Ok(output)
}
fn serde_decompress<T>(input: &[u8]) -> impl Iterator<Item = anyhow::Result<T>> + '_
where
    T: for<'de> serde::Deserialize<'de> + 'static,
{
    let decoder = match zstd::stream::read::Decoder::new(input) {
        Ok(d) => d,
        Err(e) => {
            return Box::new(std::iter::once(Err(e.into())))
                as Box<dyn Iterator<Item = _>>;
        }
    };
    let buffered = std::io::BufReader::with_capacity(128 * 1024, decoder);
    let mut de = rmp_serde::decode::Deserializer::new(buffered);
    Box::new(
        std::iter::from_fn(move || match serde::Deserialize::deserialize(&mut de) {
            Ok(item) => Some(Ok(item)),
            Err(
                rmp_serde::decode::Error::InvalidMarkerRead(ref e),
            ) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e.into())),
        }),
    )
}
fn pco_compress_nested<T>(nested_values: Vec<Vec<T>>) -> anyhow::Result<Vec<u8>>
where
    T: ::pco::data_types::Number,
{
    let mut lengths = Vec::new();
    let mut values = Vec::new();
    for vals in nested_values {
        lengths.push(vals.len() as u64);
        values.extend(vals);
    }
    let length_bytes = ::pco::standalone::simple_compress(
        &lengths,
        &::pco::ChunkConfig::default(),
    )?;
    let value_bytes = ::pco::standalone::simple_compress(
        &values,
        &::pco::ChunkConfig::default(),
    )?;
    let (length_bytes, value_bytes) = (
        serde_bytes::Bytes::new(&length_bytes),
        serde_bytes::Bytes::new(&value_bytes),
    );
    Ok(rmp_serde::to_vec(&(length_bytes, value_bytes))?)
}
fn pco_decompress_nested<T>(bytes: Vec<u8>) -> anyhow::Result<Vec<Vec<T>>>
where
    T: ::pco::data_types::Number,
{
    let (length_bytes, value_bytes): (Vec<u8>, Vec<u8>) = rmp_serde::from_slice(&bytes)?;
    let lengths = ::pco::standalone::simple_decompress::<u64>(&length_bytes)?;
    let values = ::pco::standalone::simple_decompress::<T>(&value_bytes)?;
    let mut values = values.into_iter();
    let mut nested_values = Vec::with_capacity(lengths.len());
    for length in lengths {
        nested_values.push(values.by_ref().take(length as usize).collect::<Vec<T>>());
    }
    Ok(nested_values)
}
