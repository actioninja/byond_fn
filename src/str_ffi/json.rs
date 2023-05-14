use crate::str_ffi::{error_keys, FFIError, StrArg, StrReturn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Wraps another type to represent it should be parsed as JSON, or returned as JSON.
///
/// When a type is wrapped in this, it will be parsed as JSON when passed as an argument:
/// ```
/// use byond_fn::byond_fn;
/// use byond_fn::str_ffi::json::Json;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct ExampleStruct {
///     field1: u32,
///     field2: String,
/// }
///
/// #[byond_fn]
/// fn example_fn(json: Json<ExampleStruct>) {
///     let mut unwrapped = json.into_inner();
///     // this is now a regular ExampleStruct.
///     unwrapped.field1 += 1;
/// }
/// ```
///
/// It is `repr(transparent)` so usage of this type should be zero-cost.
#[repr(transparent)]
#[derive(Debug)]
pub struct Json<T: Serialize + DeserializeOwned>(pub T);

impl<T: Serialize + DeserializeOwned> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Serialize + DeserializeOwned> From<T> for Json<T> {
    fn from(t: T) -> Self {
        Json(t)
    }
}

impl<T> StrReturn for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn to_return(self) -> Result<Option<Vec<u8>>, FFIError> {
        serde_json::to_vec(&self.0)
            .map_err(JsonError::ReturnSerialize)
            .map_err(FFIError::JsonError)
            .map(Some)
    }
}

impl<'a, T> StrArg<'a> for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn from_arg(arg: &'a str, _arg_name: &str) -> Result<Self, FFIError> {
        let deserialized: T = serde_json::from_str(arg)
            .map_err(JsonError::ArgDeserialize)
            .map_err(FFIError::JsonError)?;
        Ok(Json(deserialized))
    }
}

#[derive(Debug)]
pub enum JsonError {
    ArgDeserialize(serde_json::Error),
    ReturnSerialize(serde_json::Error),
}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};", error_keys::CLASS_JSON)?;
        match self {
            JsonError::ArgDeserialize(err) => {
                write!(f, "{};{}", error_keys::JSON_TYPE_DESERIALIZE, err)
            }
            JsonError::ReturnSerialize(err) => {
                write!(f, "{};{}", error_keys::JSON_TYPE_SERIALIZE, err)
            }
        }
    }
}

impl Error for JsonError {}

impl From<JsonError> for FFIError {
    fn from(e: JsonError) -> Self {
        FFIError::JsonError(e)
    }
}
