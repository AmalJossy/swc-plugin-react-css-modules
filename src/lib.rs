pub mod generic_names;
pub mod loader_utils;

mod auto_map_css_module;
mod config;
mod process_stylesheet;

pub use config::Config;
pub use auto_map_css_module::AutoMapCssModules;
use swc_core::{ecma::{
    ast::Program, visit::{as_folder, FoldWith}
}, plugin::metadata::TransformPluginMetadataContextKind};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};


/// An example plugin function with macro support.
/// `plugin_transform` macro interop pointers into deserialized structs, as well
/// as returning ptr back to host.
///
/// It is possible to opt out from macro by writing transform fn manually
/// if plugin need to handle low-level ptr directly via
/// `__transform_plugin_process_impl(
///     ast_ptr: *const u8, ast_ptr_len: i32,
///     unresolved_mark: u32, should_enable_comments_proxy: i32) ->
///     i32 /*  0 for success, fail otherwise.
///             Note this is only for internal pointer interop result,
///             not actual transform result */`
///
/// This requires manual handling of serialization / deserialization from ptrs.
/// Refer swc_plugin_macro to see how does it work internally.
#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config: Config = serde_json::from_str(
        &metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config"),
    )
    .expect("invalid config");

    let filepath = metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .expect("failed to get filepath");

    let cwd = metadata
        .get_context(&TransformPluginMetadataContextKind::Cwd)
        .expect("failed to get cwd");

    program.fold_with(&mut as_folder(AutoMapCssModules::new(
        cwd.as_str(),
        filepath.as_str(),
        config,
    )))
}

// An example to test plugin transform.
// Recommended strategy to test plugin's transform is verify
// the Visitor's behavior, instead of trying to run `process_transform` with mocks
// unless explicitly required to do so.
// test!(
//     Default::default(),
//     |_| as_folder(AutoMapCssModules::new(env::current_dir().unwrap().to_str().unwrap(), "src/lib.rs")),
//     boo,
//     r#"foo === bar;"#
// );