//! Extension framework macros: `impl_extension`!, `data_field`!, `test_datasource_extension`!.

/// Macro to reduce boilerplate when implementing the Extension trait.
///
/// Single variant with optional `command_names` arm.
///
/// Generates the simple accessor methods (name, description, shortname, types,
/// `data_provider_name`) from the provided constants, and accepts custom blocks
/// for `register_commands` and `register_data_providers`.
///
/// # Example
///
/// ```text
/// impl_extension! {
///     MyExtension,
///     name: NAME,
///     description: DESCRIPTION,
///     shortname: SHORTNAME,
///     types: ExtensionType::DATASOURCE,
///     data_provider_name: Some(DATA_PROVIDER_NAME),
///     register_commands: |_self, _registry| {},
///     register_data_providers: |_self, registry| {
///         // `register` is `#[must_use]`: it hands back a provider it
///         // rejected as a duplicate. `let _ =` accepts the drop.
///         let _ = registry.register(DATA_PROVIDER_NAME, Box::new(MyProvider));
///     },
/// }
/// ```
#[macro_export]
macro_rules! impl_extension {
    // Internal rule: shared accessor methods
    (@accessors $name:expr, $desc:expr, $short:expr, $types:expr, $dp:expr $(, stack: $stack:expr)? $(, command_names: $cn:expr)?) => {
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

    // Internal rule: the auto-registration static shared by both factory arms.
    //
    // linkme is reached through this crate's hidden re-export, so invoking
    // crates need no direct `linkme` dependency. linkme's distributed_slice
    // expansion emits `#[link_section]` (an unsafe attribute wrapper) on the
    // static — unsafe tokens the compiler counts into the invoking crate. The
    // allow must live here, on the generated item: rustc ignores `#[allow]`
    // applied to the macro *invocation*. In crates where `unsafe_code` is not
    // denied this is inert. The expansion requires `deny(unsafe_code)` or a
    // less restrictive level in the invoking crate: under `forbid(unsafe_code)`
    // the generated `#[allow]` is itself rejected (E0451 — an allow cannot
    // override a forbid).
    (@factory $factory_ident:ident = $factory_fn:expr) => {
        #[allow(unsafe_code)]
        #[$crate::__private::linkme::distributed_slice($crate::EXTENSION_REGISTRY)]
        #[linkme(crate = $crate::__private::linkme)]
        static $factory_ident: $crate::ExtensionFactory = $factory_fn;
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
        $crate::impl_extension! {
            $struct,
            name: $name,
            description: $desc,
            shortname: $short,
            types: $types,
            $(stack: $stack,)?
            $(command_names: $cn,)?
            data_provider_name: $dp,
            register_commands: |$self_cmd, $reg_cmd| $cmd_body,
            register_data_providers: |$self_dp, $reg_dp| $dp_body,
        }
        $crate::impl_extension!(@factory $factory_ident = $factory_fn);
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
            $crate::impl_extension!(@accessors $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
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
        $crate::impl_extension! {
            $struct,
            name: $name,
            description: $desc,
            shortname: $short,
            types: $types,
            $(stack: $stack,)?
            $(command_names: $cn,)?
            data_provider_name: $dp,
            register_data_providers: |$self_dp, $reg_dp| $dp_body,
        }
        $crate::impl_extension!(@factory $factory_ident = $factory_fn);
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
            $crate::impl_extension!(@accessors $name, $desc, $short, $types, $dp $(, stack: $stack)? $(, command_names: $cn)?);
            fn register_commands(&self, _registry: &mut $crate::CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }
    };
}

/// Shorthand macro for constructing a [`crate::data::DataField`].
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
        $crate::DataField::new($name, $type_name, $description)
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
