extern crate proc_macro;

mod attrs;
mod shape;
mod symbol;

use std::{env, path::PathBuf};

pub use attrs::*;
use proc_macro::TokenStream;
use quote::quote;
pub use shape::*;
pub use symbol::*;
use toml::{map::Map, Value};

pub struct ZmaxionManifest {
    manifest: Map<String, Value>,
}

impl Default for ZmaxionManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    let manifest = std::fs::read_to_string(path).unwrap();
                    toml::from_str(&manifest).unwrap()
                })
                .unwrap(),
        }
    }
}

impl ZmaxionManifest {
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        const ZMAXION: &str = "zmaxion";
        const ZMAXION_INTERNAL: &str = "zmaxion_internal";

        fn dep_package(dep: &Value) -> Option<&str> {
            if dep.as_str().is_some() {
                None
            } else {
                dep.as_table()
                    .unwrap()
                    .get("package")
                    .map(|name| name.as_str().unwrap())
            }
        }

        let find_in_deps = |deps: &Map<String, Value>| -> Option<syn::Path> {
            let package = if let Some(dep) = deps.get(name) {
                return Some(Self::parse_str(dep_package(dep).unwrap_or(name)));
            } else if let Some(dep) = deps.get(ZMAXION) {
                dep_package(dep).unwrap_or(ZMAXION)
            } else if let Some(dep) = deps.get(ZMAXION_INTERNAL) {
                dep_package(dep).unwrap_or(ZMAXION_INTERNAL)
            } else {
                return None;
            };

            let mut path = Self::parse_str::<syn::Path>(package);
            if let Some(module) = name.strip_prefix("bevy_") {
                path.segments.push(Self::parse_str(module));
            }
            Some(path)
        };

        let deps = self
            .manifest
            .get("dependencies")
            .map(|deps| deps.as_table().unwrap());
        let deps_dev = self
            .manifest
            .get("dev-dependencies")
            .map(|deps| deps.as_table().unwrap());

        deps.and_then(find_in_deps)
            .or_else(|| deps_dev.and_then(find_in_deps))
    }

    pub fn get_path(&self, name: &str) -> syn::Path {
        self.maybe_get_path(name)
            .unwrap_or_else(|| Self::parse_str(name))
    }

    pub fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
        syn::parse(path.parse::<TokenStream>().unwrap()).unwrap()
    }
}
