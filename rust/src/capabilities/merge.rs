//! Merge / Replace for facade set* APIs.

use crate::capabilities::errors::CapabilityError;
use crate::capabilities::generated::SetMode;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn apply_set_mode<T>(
    current: &T,
    incoming: &T,
    mode: SetMode,
) -> Result<T, CapabilityError>
where
    T: Serialize + DeserializeOwned + Clone,
{
    match mode {
        SetMode::Replace => Ok(incoming.clone()),
        SetMode::Merge => {
            let mut cur = serde_json::to_value(current)
                .map_err(|e| CapabilityError::validation_error(e.to_string()))?;
            let inc = serde_json::to_value(incoming)
                .map_err(|e| CapabilityError::validation_error(e.to_string()))?;
            if let (serde_json::Value::Object(c), serde_json::Value::Object(i)) = (&mut cur, inc)
            {
                for (k, v) in i {
                    c.insert(k, v);
                }
            }
            serde_json::from_value(cur)
                .map_err(|e| CapabilityError::validation_error(e.to_string()))
        }
    }
}
