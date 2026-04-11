//! Extension framework macros: impl_extension!, data_field!, test_datasource_extension!.

/// Macro to reduce boilerplate when implementing the Extension trait.
///
/// DUP-002: Single variant with optional `command_names` arm.
///
/// Generates the simple accessor methods (name, description, shortname, types,
/// data_provider_name) from the provided constants, and accepts custom blocks
/// for register_commands and register_data_providers.
///
/// # Example
///
/// ```ignore
/// impl_extension! {
///     MyExtension,
///     name: NAME,
///     description: DESCRIPTION,
///     shortname: SHORTNAME,
///     types: ExtensionType::DATASOURCE,
///     data_provider_name: Some(DATA_PROVIDER_NAME),
///     register_commands: |_self, _registry| {},
///     register_data_providers: |_self, registry| {
///         registry.register(DATA_PROVIDER_NAME, Box::new(MyProvider));
///     },
/// }
/// ```
#[macro_export]
macro_rules! impl_extension {
    // Internal rule: shared accessor methods (DUP-036 fix)
    (@accessors $struct:ty, $name:expr, $desc:expr, $short:expr, $types:expr, $dp:expr $(, stack: $stack:expr)? $(, command_names: $cn:expr)?) => {
        fn name(&self) -> &'static str {
            $name
        }
        fn description(&self) -> &'static str {
            $desc
        }
        fn shortname(&self) -> &'static str {
            $short
        }
        fn types(&self) -> $crate::ExtensionType {
            $types
        }
        $(
            fn stack(&self) -> Option<$crate::Stack> {
                $stack
            }
        )?
        $(
            fn command_names(&self) -> &'static [&'static str] {
                $cn
            }
        )?
        fn data_provider_name(&self) -> Option<&'static str> {
            $dp
        }
    };

    // Full form with register_commands + factory (auto-registration)
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(stack: $stack:expr,)?
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_commands: |$self_cmd:ident, $reg_cmd:ident| $cmd_body:block,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block,
        factory: $factory_ident:ident = $factory_fn:expr $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
            fn register_commands(&self, registry: &mut $crate::CommandRegistry) {
                let $self_cmd = self;
                let $reg_cmd = registry;
                $cmd_body
            }
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }

        #[linkme::distributed_slice($crate::EXTENSION_REGISTRY)]
        static $factory_ident: $crate::ExtensionFactory = $factory_fn;
    };

    // Full form with register_commands (no factory — legacy)
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(stack: $stack:expr,)?
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_commands: |$self_cmd:ident, $reg_cmd:ident| $cmd_body:block,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
            fn register_commands(&self, registry: &mut $crate::CommandRegistry) {
                let $self_cmd = self;
                let $reg_cmd = registry;
                $cmd_body
            }
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }
    };

    // Short form with factory (auto-registration, no register_commands)
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(stack: $stack:expr,)?
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block,
        factory: $factory_ident:ident = $factory_fn:expr $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
            fn register_commands(&self, _registry: &mut $crate::CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }

        #[linkme::distributed_slice($crate::EXTENSION_REGISTRY)]
        static $factory_ident: $crate::ExtensionFactory = $factory_fn;
    };

    // Short form without register_commands (no factory — legacy)
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(stack: $stack:expr,)?
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
            fn register_commands(&self, _registry: &mut $crate::CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }
    };
}

/// Shorthand macro for constructing a [`DataField`].
///
/// Reduces verbose struct initialization from 5 lines to 1.
///
/// # Example
///
/// ```ignore
/// use ops_extension::data_field;
///
/// let fields = vec![
///     data_field!("name", "str", "Package name"),
///     data_field!("version", "str", "Package version string"),
/// ];
/// ```
#[macro_export]
macro_rules! data_field {
    ($name:expr, $type_name:expr, $description:expr) => {
        $crate::DataField {
            name: $name,
            type_name: $type_name,
            description: $description,
        }
    };
}

/// Macro to generate standard extension registration tests for datasource extensions.
///
/// Generates two tests:
/// - `extension_name`: Verifies the extension returns the expected name
/// - `extension_registers_data_provider`: Verifies the extension registers a data provider
///
/// # Example
///
/// ```ignore
/// ops_extension::test_datasource_extension!(
///     MetadataExtension,
///     name: "metadata",
///     data_provider: "metadata"
/// );
/// ```
#[macro_export]
macro_rules! test_datasource_extension {
    ($ext:expr, name: $name:expr, data_provider: $dp:expr) => {
        #[test]
        fn extension_name() {
            assert_eq!($crate::Extension::name(&$ext), $name);
        }

        #[test]
        fn extension_registers_data_provider() {
            let mut registry = $crate::DataRegistry::new();
            $crate::Extension::register_data_providers(&$ext, &mut registry);
            assert!(registry.get($dp).is_some());
        }
    };
}
