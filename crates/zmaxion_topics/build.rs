extern crate toml_edit;

use std::{env, ffi::OsStr, fmt::Write, fs, io, path::Path};

use ident_case::RenameRule;
use toml_edit::{Document, Formatted, InlineTable, Item, TomlError, Value};

fn main() -> io::Result<()> {
    let env_name = "ZMAXION_FRIEND_";
    let config = fs::read_to_string("base.toml")?;
    let mut base_doc = config
        .parse::<Document>()
        .expect("invalid base.toml, this is a bug");
    let prev_config = fs::read_to_string("Cargo.toml")?;
    let mut prelude_src = "".to_string();
    let mut exports_src = "".to_string();
    let mut writer_src = "".to_string();
    let mut reader_src = "".to_string();
    let mut factory_src = "".to_string();

    for (var_name, crate_path) in env::vars().filter(|x| x.0.starts_with(env_name)) {
        let crate_path = Path::new(&crate_path);
        let cargo_toml_path = crate_path.join("Cargo.toml");
        let mut friend_name_snake = var_name.strip_prefix(env_name).unwrap().to_string();
        friend_name_snake.make_ascii_lowercase();
        let module_name = &friend_name_snake;

        println!("cargo:rerun-if-changed={}", crate_path.display());

        let friend_toml = fs::read_to_string(&cargo_toml_path)?;
        let friend_doc = friend_toml
            .parse::<Document>()
            .map_err(|e| {
                panic!(
                    "invalid {} of {}, caused by {:?}",
                    cargo_toml_path.display(),
                    var_name,
                    e
                )
            })
            .unwrap();
        for (dep, version) in friend_doc["dependencies"].as_table().expect(&format!(
            "dependencies in '{}' should be a table",
            cargo_toml_path.display()
        )) {
            base_doc["dependencies"][dep] = version.clone();
        }

        let crate_name = friend_doc["package"]["name"].as_str().expect(&format!(
            "{}: package name should be a string",
            cargo_toml_path.display()
        ));
        let mut table = InlineTable::new();
        table.insert(
            "path",
            Value::String(Formatted::new(
                crate_path.as_os_str().to_str().unwrap().to_string(),
            )),
        );
        base_doc["dependencies"][crate_name] = Item::Value(Value::InlineTable(table));
        let friend_name_pascal = RenameRule::PascalCase.apply_to_field(friend_name_snake);
        #[allow(unused_must_use)]
        {
            write!(&mut exports_src, "pub use {};", crate_name);
            write!(&mut prelude_src, "\tpub use {}::prelude::*;\n", crate_name);
            write!(
                &mut reader_src,
                "\t{}({}::__reader_state_type!()),\n",
                friend_name_pascal, crate_name
            );
            write!(
                &mut writer_src,
                "\t{}({}::__writer_state_type!()),\n",
                friend_name_pascal, crate_name
            );
            write!(
                &mut factory_src,
                "\targs.commands.insert({}::__default_topic!());\n",
                crate_name
            );
        }
    }
    let src = format!(
        r#"
use zmaxion_rt::AsyncMutex;
use zmaxion_core::models::TopicSpawnerArgs;
use bevy_ecs::system::Resource;
use std::sync::Arc;

{exports_src}

pub mod prelude {{
{prelude_src}}}

pub struct ReaderState<T: Resource>(pub Arc<AsyncMutex<ReaderStateEnum<T>>>);
pub struct WriterState<T: Resource>(pub Arc<AsyncMutex<WriterStateEnum<T>>>);

pub enum ReaderStateEnum<T> {{
{reader_src}}}
pub enum WriterStateEnum<T> {{
{writer_src}}}
pub fn topic_factory<T: Resource>(args: &mut TopicSpawnerArgs) {{
{factory_src}}}
"#
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ZMAXION_FRIENDS_RERUN");
    let base_config = base_doc.to_string();
    if prev_config != base_config {
        println!(
            "cargo:warning=Dependencies injected, if you encounter an error run cargo build again."
        );
    }
    fs::write("Cargo.toml", base_config)?;
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("topics.rs");
    fs::write(dest_path, src)?;

    Ok(())
}
