#[allow(warnings)]
#[allow(unsafe_op_in_unsafe_fn)]
mod bindings;

use aura_wasm_guest::{AuraValue, ErrorCode};
use bindings::exports::aura::runtime::plugin::{Guest, PluginError};
use std::sync::atomic::{AtomicU8, Ordering};

const CREATED: u8 = 0;
const LOADED: u8 = 1;
const ENABLED: u8 = 2;

static STATE: AtomicU8 = AtomicU8::new(CREATED);

struct LaunchHook;

impl Guest for LaunchHook {
    fn load() -> Result<(), PluginError> {
        transition(CREATED, LOADED)
    }

    fn enable() -> Result<(), PluginError> {
        transition(LOADED, ENABLED)
    }

    fn invoke(
        operation: String,
        input: Vec<u8>,
        _callback_id: u64,
    ) -> Result<Vec<u8>, PluginError> {
        if STATE.load(Ordering::Acquire) != ENABLED {
            return Err(plugin_error("invalid-state", "plugin is not enabled"));
        }
        if operation != "hook.before-game-launch" {
            return Err(plugin_error("invalid-argument", "unsupported operation"));
        }

        let event = AuraValue::from_wire(&input).map_err(bridge_error)?;
        let event_entries = require_map(event, "Hook event must be a map")?;
        require_integer(&event_entries, "contractVersion", 1)?;
        require_string(&event_entries, "point", "before-game-launch")?;
        let mut data = take_field(event_entries, "data")?;
        let data_entries = map_mut(&mut data, "Hook data must be a map")?;
        let plan = field_mut(data_entries, "plan")?;
        let plan_entries = map_mut(plan, "launch plan must be a map")?;
        let command = field_mut(plan_entries, "command")?;
        let command_entries = map_mut(command, "launch command must be a map")?;

        if !field_is_string(command_entries, "mode", "structured-java") {
            return unchanged_result();
        }
        let arguments = field_mut(command_entries, "jvmArguments")?;
        let AuraValue::Array(arguments) = arguments else {
            return Err(plugin_error(
                "invalid-argument",
                "jvmArguments must be an array",
            ));
        };
        arguments.push(AuraValue::String(
            "-Daura.example.wasm-hook=true".to_owned(),
        ));

        AuraValue::Map(vec![
            ("contractVersion".to_owned(), AuraValue::Integer(1)),
            ("action".to_owned(), AuraValue::String("replace".to_owned())),
            ("data".to_owned(), data),
            ("protectedSecrets".to_owned(), AuraValue::Map(Vec::new())),
        ])
        .to_wire()
        .map_err(bridge_error)
    }

    fn disable() -> Result<(), PluginError> {
        transition(ENABLED, LOADED)
    }

    fn unload() -> Result<(), PluginError> {
        transition(LOADED, CREATED)
    }
}

fn transition(from: u8, to: u8) -> Result<(), PluginError> {
    STATE
        .compare_exchange(from, to, Ordering::AcqRel, Ordering::Acquire)
        .map(|_| ())
        .map_err(|_| plugin_error("invalid-state", "invalid plugin lifecycle transition"))
}

fn require_map(value: AuraValue, message: &str) -> Result<Vec<(String, AuraValue)>, PluginError> {
    match value {
        AuraValue::Map(entries) => Ok(entries),
        _ => Err(plugin_error("invalid-argument", message)),
    }
}

fn map_mut<'a>(
    value: &'a mut AuraValue,
    message: &str,
) -> Result<&'a mut Vec<(String, AuraValue)>, PluginError> {
    match value {
        AuraValue::Map(entries) => Ok(entries),
        _ => Err(plugin_error("invalid-argument", message)),
    }
}

fn field_mut<'a>(
    entries: &'a mut [(String, AuraValue)],
    name: &str,
) -> Result<&'a mut AuraValue, PluginError> {
    entries
        .iter_mut()
        .find_map(|(key, value)| (key == name).then_some(value))
        .ok_or_else(|| plugin_error("invalid-argument", "required Hook field is missing"))
}

fn take_field(mut entries: Vec<(String, AuraValue)>, name: &str) -> Result<AuraValue, PluginError> {
    let index = entries
        .iter()
        .position(|(key, _)| key == name)
        .ok_or_else(|| plugin_error("invalid-argument", "required Hook field is missing"))?;
    Ok(entries.remove(index).1)
}

fn require_integer(
    entries: &[(String, AuraValue)],
    name: &str,
    expected: i64,
) -> Result<(), PluginError> {
    if entries
        .iter()
        .any(|(key, value)| key == name && value == &AuraValue::Integer(expected))
    {
        Ok(())
    } else {
        Err(plugin_error(
            "invalid-argument",
            "invalid Hook contract version",
        ))
    }
}

fn require_string(
    entries: &[(String, AuraValue)],
    name: &str,
    expected: &str,
) -> Result<(), PluginError> {
    if field_is_string(entries, name, expected) {
        Ok(())
    } else {
        Err(plugin_error("invalid-argument", "invalid Hook point"))
    }
}

fn field_is_string(entries: &[(String, AuraValue)], name: &str, expected: &str) -> bool {
    entries.iter().any(
        |(key, value)| matches!(value, AuraValue::String(actual) if key == name && actual == expected),
    )
}

fn unchanged_result() -> Result<Vec<u8>, PluginError> {
    AuraValue::Map(vec![
        ("contractVersion".to_owned(), AuraValue::Integer(1)),
        (
            "action".to_owned(),
            AuraValue::String("unchanged".to_owned()),
        ),
    ])
    .to_wire()
    .map_err(bridge_error)
}

fn bridge_error(error: aura_wasm_guest::Error) -> PluginError {
    let code = match error.code() {
        ErrorCode::InvalidArgument => "invalid-argument",
        ErrorCode::InvalidResult => "invalid-result",
        _ => "internal",
    };
    plugin_error(code, "invalid Bridge Value v1 payload")
}

fn plugin_error(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

bindings::export!(LaunchHook with_types_in bindings);
