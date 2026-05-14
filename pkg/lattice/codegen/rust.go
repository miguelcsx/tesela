// Rust renderer — emits typed structs + a reqwest-based client.

package codegen

import (
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type rustRenderer struct{}

func (*rustRenderer) Language() string { return "rust" }

func (r *rustRenderer) Render(snap *types.Ontology) (map[string]string, error) {
	files := make(map[string]string, 3)
	files["Cargo.toml"] = `[package]
name = "lattice-sdk"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
` + "\n"
	files["src/lib.rs"] = renderRustModule(snap)
	files["README.md"] = "# Lattice Rust SDK\n"
	return files, nil
}

func renderRustModule(snap *types.Ontology) string {
	var b strings.Builder
	b.WriteString(`//! Auto-generated Rust SDK for Lattice.
use serde::{Deserialize, Serialize};

`)
	for _, ot := range snap.ObjectTypes {
		b.WriteString("#[derive(Debug, Clone, Serialize, Deserialize)]\n")
		b.WriteString(fmt.Sprintf("pub struct %s {\n", tsTypeName(string(ot.APIName))))
		for _, p := range ot.Properties {
			b.WriteString(fmt.Sprintf("    pub %s: Option<%s>,\n", p.APIName, rustType(p.DataType)))
		}
		b.WriteString("}\n\n")
	}
	b.WriteString(`pub struct Client {
    pub base_url: String,
    pub token: String,
    pub workspace: String,
    pub http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: String, token: String, workspace: String) -> Self {
        Self { base_url, token, workspace, http: reqwest::Client::new() }
    }
    pub async fn search(&self, ty: &str, spec: serde_json::Value) -> Result<serde_json::Value, reqwest::Error> {
        let url = format!("{}/v1/workspaces/{}/objects/{}:search", self.base_url, self.workspace, ty);
        self.http.post(&url).bearer_auth(&self.token).json(&spec).send().await?.json().await
    }
}
`)
	return b.String()
}

func rustType(dt types.DataType) string {
	switch dt {
	case types.DataTypeInteger:
		return "i32"
	case types.DataTypeBigInt:
		return "i64"
	case types.DataTypeFloat, types.DataTypeDecimal:
		return "f64"
	case types.DataTypeBoolean:
		return "bool"
	default:
		return "String"
	}
}
