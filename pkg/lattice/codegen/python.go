// Python renderer — emits @dataclass classes + a typed client.

package codegen

import (
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type pythonRenderer struct{}

func (*pythonRenderer) Language() string { return "python" }

func (r *pythonRenderer) Render(snap *types.Ontology) (map[string]string, error) {
	files := make(map[string]string, 3)
	files["pyproject.toml"] = `[project]
name = "lattice-sdk"
version = "0.1.0"
` + "\n"
	files["lattice/__init__.py"] = renderPythonModule(snap)
	files["README.md"] = "# Lattice SDK\n"
	return files, nil
}

func renderPythonModule(snap *types.Ontology) string {
	var b strings.Builder
	b.WriteString(`"""Auto-generated Python SDK for Lattice."""
from dataclasses import dataclass
from typing import Any, Dict, Optional
import httpx

`)
	for _, ot := range snap.ObjectTypes {
		b.WriteString(fmt.Sprintf("@dataclass\nclass %s:\n", tsTypeName(string(ot.APIName))))
		if len(ot.Properties) == 0 {
			b.WriteString("    pass\n\n")
			continue
		}
		for _, p := range ot.Properties {
			b.WriteString(fmt.Sprintf("    %s: %s = None\n", p.APIName, pyType(p.DataType)))
		}
		b.WriteString("\n")
	}
	b.WriteString(`class LatticeClient:
    def __init__(self, base_url: str, token: str, workspace: str):
        self.base_url, self.token, self.workspace = base_url, token, workspace
        self._http = httpx.Client(headers={"Authorization": f"Bearer {token}"}, timeout=60)

    def search(self, type_: str, spec: Dict[str, Any]) -> Any:
        r = self._http.post(f"{self.base_url}/v1/workspaces/{self.workspace}/objects/{type_}:search", json=spec)
        r.raise_for_status(); return r.json()

    def get(self, type_: str, pk: str) -> Any:
        r = self._http.get(f"{self.base_url}/v1/workspaces/{self.workspace}/objects/{type_}/{pk}")
        r.raise_for_status(); return r.json()

    def execute(self, action: str, input_: Dict[str, Any], idem: Optional[str] = None) -> Any:
        headers = {"Idempotency-Key": idem} if idem else {}
        r = self._http.post(
            f"{self.base_url}/v1/workspaces/{self.workspace}/actions/{action}:execute",
            json={"input": input_}, headers=headers,
        )
        r.raise_for_status(); return r.json()
`)
	return b.String()
}

func pyType(dt types.DataType) string {
	switch dt {
	case types.DataTypeInteger, types.DataTypeBigInt:
		return "Optional[int]"
	case types.DataTypeFloat, types.DataTypeDecimal:
		return "Optional[float]"
	case types.DataTypeBoolean:
		return "Optional[bool]"
	default:
		return "Optional[str]"
	}
}
