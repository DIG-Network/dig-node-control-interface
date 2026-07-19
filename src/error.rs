//! The canonical control-plane error taxonomy.
//!
//! One definition point for every error code a dig-node control method emits, the [`ControlError`]
//! envelope (`{code, message, data:{code, origin}}`), and the constructor both sides use so every
//! error carries a machine-branchable `data.code` (UPPER_SNAKE) and `data.origin`.
//!
//! The numeric values are the SAME published wire contract as dig-node's `meta::ErrorCode` and the
//! canonical dig-rpc `-320xx` control range — they never change once assigned. A client keys its UX
//! off `data.code` (the stable symbol), never the human `message`.
//!
//! | Code | Variant | Origin | Meaning |
//! |---|---|---|---|
//! | `-32700` | [`ParseError`](ControlErrorCode::ParseError) | shell | request body was not valid JSON |
//! | `-32600` | [`InvalidRequest`](ControlErrorCode::InvalidRequest) | shell | not a single JSON-RPC object |
//! | `-32601` | [`MethodNotFound`](ControlErrorCode::MethodNotFound) | boundary | control method is not resolved |
//! | `-32602` | [`InvalidParams`](ControlErrorCode::InvalidParams) | node | missing/malformed params |
//! | `-32000` | [`DispatchFailed`](ControlErrorCode::DispatchFailed) | shell | the node failed to dispatch |
//! | `-32030` | [`Unauthorized`](ControlErrorCode::Unauthorized) | shell | called without a valid control token |
//! | `-32031` | [`NotSupported`](ControlErrorCode::NotSupported) | shell | control op unsupported on this build |
//! | `-32032` | [`ControlError`](ControlErrorCode::ControlError) | shell | control op failed at runtime |

use serde::{Deserialize, Serialize};

/// A canonical control-plane error code.
///
/// `#[non_exhaustive]` so adding a code in a minor release is additive; downstream matches must use
/// a `_ => …` arm.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlErrorCode {
    /// `-32700` — request body was not valid JSON.
    ParseError,
    /// `-32600` — the request was not a single JSON-RPC object (batches are unsupported).
    InvalidRequest,
    /// `-32601` — the control method is not resolved by the node.
    MethodNotFound,
    /// `-32602` — invalid or missing method parameters.
    InvalidParams,
    /// `-32000` — the node failed to dispatch a well-formed control call.
    DispatchFailed,
    /// `-32030` — a `control.*` method was called without a valid local control token.
    Unauthorized,
    /// `-32031` — the control operation is not supported on this node build (e.g. §21 sync with no
    /// identity loaded, or a capability the running engine does not expose).
    NotSupported,
    /// `-32032` — a control operation failed at runtime (e.g. a config write or cache op errored).
    ControlError,
}

impl ControlErrorCode {
    /// The numeric JSON-RPC error code.
    pub const fn code(self) -> i64 {
        match self {
            ControlErrorCode::ParseError => -32700,
            ControlErrorCode::InvalidRequest => -32600,
            ControlErrorCode::MethodNotFound => -32601,
            ControlErrorCode::InvalidParams => -32602,
            ControlErrorCode::DispatchFailed => -32000,
            ControlErrorCode::Unauthorized => -32030,
            ControlErrorCode::NotSupported => -32031,
            ControlErrorCode::ControlError => -32032,
        }
    }

    /// The stable UPPER_SNAKE symbolic name a client branches on. Never derived from the message.
    pub const fn name(self) -> &'static str {
        match self {
            ControlErrorCode::ParseError => "PARSE_ERROR",
            ControlErrorCode::InvalidRequest => "INVALID_REQUEST",
            ControlErrorCode::MethodNotFound => "METHOD_NOT_FOUND",
            ControlErrorCode::InvalidParams => "INVALID_PARAMS",
            ControlErrorCode::DispatchFailed => "DISPATCH_FAILED",
            ControlErrorCode::Unauthorized => "UNAUTHORIZED",
            ControlErrorCode::NotSupported => "NOT_SUPPORTED",
            ControlErrorCode::ControlError => "CONTROL_ERROR",
        }
    }

    /// Where the error originates, mirroring dig-node's classification.
    pub const fn origin(self) -> &'static str {
        match self {
            ControlErrorCode::MethodNotFound => "boundary",
            ControlErrorCode::InvalidParams => "node",
            _ => "shell",
        }
    }

    /// A one-line description for the error catalogue.
    pub const fn description(self) -> &'static str {
        match self {
            ControlErrorCode::ParseError => "Request body was not valid JSON.",
            ControlErrorCode::InvalidRequest => {
                "Request was not a single JSON-RPC object (batch arrays are not supported)."
            }
            ControlErrorCode::MethodNotFound => "The control method is not resolved by the node.",
            ControlErrorCode::InvalidParams => "Invalid or missing method parameters.",
            ControlErrorCode::DispatchFailed => "The node failed to dispatch the control request.",
            ControlErrorCode::Unauthorized => {
                "A control.* method was called without a valid local control token."
            }
            ControlErrorCode::NotSupported => {
                "The requested control operation is not supported on this node build."
            }
            ControlErrorCode::ControlError => "A control operation failed at runtime.",
        }
    }

    /// Resolve a numeric wire code back to its variant, or `None` for an uncatalogued code.
    pub fn from_code(code: i64) -> Option<ControlErrorCode> {
        ControlErrorCode::ALL
            .iter()
            .copied()
            .find(|c| c.code() == code)
    }

    /// Every catalogued code, for the error-catalogue document + drift tests.
    pub const ALL: &'static [ControlErrorCode] = &[
        ControlErrorCode::ParseError,
        ControlErrorCode::InvalidRequest,
        ControlErrorCode::MethodNotFound,
        ControlErrorCode::InvalidParams,
        ControlErrorCode::DispatchFailed,
        ControlErrorCode::Unauthorized,
        ControlErrorCode::NotSupported,
        ControlErrorCode::ControlError,
    ];
}

/// The machine-branchable inner data on a control error: the stable symbol + its origin.
///
/// Serializes to `{"code": "UNAUTHORIZED", "origin": "shell"}`, matching dig-node's `error.data`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlErrorData {
    /// The stable UPPER_SNAKE symbol (e.g. `"UNAUTHORIZED"`) — see [`ControlErrorCode::name`].
    pub code: String,
    /// Where the error originated (`shell` / `node` / `boundary`) — see [`ControlErrorCode::origin`].
    pub origin: String,
}

/// A control-plane error, serialized as the JSON-RPC `error` object
/// (`{code, message, data:{code, origin}}`).
///
/// [`code`](ControlError::code) is the numeric wire value; [`data`](ControlError::data) carries the
/// stable symbol + origin a client actually branches on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlError {
    /// The numeric JSON-RPC error code.
    pub code: i64,
    /// A human-readable message (never contract-stable — for display/logs only).
    pub message: String,
    /// The machine-branchable symbol + origin.
    pub data: ControlErrorData,
}

impl ControlError {
    /// Build an error from a catalogued [`ControlErrorCode`] and a human message.
    pub fn of(code: ControlErrorCode, message: impl Into<String>) -> ControlError {
        ControlError {
            code: code.code(),
            message: message.into(),
            data: ControlErrorData {
                code: code.name().to_string(),
                origin: code.origin().to_string(),
            },
        }
    }

    /// The catalogued code, resolved from the numeric wire value (`None` if uncatalogued).
    pub fn code_enum(&self) -> Option<ControlErrorCode> {
        ControlErrorCode::from_code(self.code)
    }
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.data.code, self.code, self.message)
    }
}

impl std::error::Error for ControlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_symbols_and_origins_are_stable() {
        assert_eq!(ControlErrorCode::Unauthorized.code(), -32030);
        assert_eq!(ControlErrorCode::NotSupported.code(), -32031);
        assert_eq!(ControlErrorCode::ControlError.code(), -32032);
        assert_eq!(ControlErrorCode::InvalidParams.name(), "INVALID_PARAMS");
        assert_eq!(ControlErrorCode::InvalidParams.origin(), "node");
        assert_eq!(ControlErrorCode::MethodNotFound.origin(), "boundary");
        assert_eq!(ControlErrorCode::Unauthorized.origin(), "shell");
    }

    #[test]
    fn from_code_round_trips_every_code() {
        for &c in ControlErrorCode::ALL {
            assert_eq!(ControlErrorCode::from_code(c.code()), Some(c));
        }
        assert_eq!(ControlErrorCode::from_code(0), None);
    }

    #[test]
    fn every_code_has_a_description() {
        for &c in ControlErrorCode::ALL {
            assert!(!c.description().is_empty());
        }
    }

    #[test]
    fn error_serializes_to_the_canonical_json_shape() {
        let e = ControlError::of(ControlErrorCode::Unauthorized, "no token");
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["code"], -32030);
        assert_eq!(v["message"], "no token");
        assert_eq!(v["data"]["code"], "UNAUTHORIZED");
        assert_eq!(v["data"]["origin"], "shell");
        assert_eq!(e.code_enum(), Some(ControlErrorCode::Unauthorized));
    }

    #[test]
    fn display_is_symbol_code_and_message() {
        let e = ControlError::of(ControlErrorCode::ControlError, "boom");
        assert_eq!(e.to_string(), "CONTROL_ERROR (-32032): boom");
    }
}
