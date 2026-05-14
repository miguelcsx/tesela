// TypeScript renderer — emits a typed client + interfaces.

package codegen

import (
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type typescriptRenderer struct{}

func (*typescriptRenderer) Language() string { return "typescript" }

func (r *typescriptRenderer) Render(snap *types.Ontology) (map[string]string, error) {
	files := make(map[string]string, 4)
	files["package.json"] = `{"name":"lattice-sdk","version":"0.1.0","main":"index.js","types":"index.d.ts"}` + "\n"
	files["index.ts"] = renderTypescriptIndex(snap)
	files["README.md"] = "# Lattice SDK\n\nGenerated from ontology " + string(snap.Workspace.APIName) + ".\n"
	return files, nil
}

func renderTypescriptIndex(snap *types.Ontology) string {
	var b strings.Builder
	b.WriteString("// Auto-generated TypeScript SDK for Lattice\n\n")
	for _, ot := range snap.ObjectTypes {
		b.WriteString(fmt.Sprintf("export interface %s {\n", tsTypeName(string(ot.APIName))))
		for _, p := range ot.Properties {
			b.WriteString(fmt.Sprintf("  %s: %s;\n", p.APIName, tsType(p.DataType)))
		}
		b.WriteString("}\n\n")
	}
	b.WriteString(`export class LatticeClient {
  constructor(private baseURL: string, private token: string, private workspace: string) {}
  private async req(method: string, path: string, body?: any) {
    const r = await fetch(this.baseURL + path, {
      method, headers: { "Authorization": "Bearer " + this.token, "Content-Type": "application/json" },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!r.ok) throw new Error("HTTP " + r.status + ": " + (await r.text()));
    return r.json();
  }
  search(type: string, spec: any) { return this.req("POST", ` + "`/v1/workspaces/${this.workspace}/objects/${type}:search`" + `, spec); }
  get(type: string, pk: string) { return this.req("GET", ` + "`/v1/workspaces/${this.workspace}/objects/${type}/${pk}`" + `); }
  execute(action: string, input: any, idem?: string) { return this.req("POST", ` + "`/v1/workspaces/${this.workspace}/actions/${action}:execute`" + `, { input }); }
}
`)
	return b.String()
}

func tsType(dt types.DataType) string {
	switch dt {
	case types.DataTypeInteger, types.DataTypeBigInt, types.DataTypeFloat, types.DataTypeDecimal:
		return "number"
	case types.DataTypeBoolean:
		return "boolean"
	default:
		return "string"
	}
}

func tsTypeName(s string) string {
	parts := strings.Split(s, "_")
	for i, p := range parts {
		if len(p) > 0 {
			parts[i] = strings.ToUpper(p[:1]) + p[1:]
		}
	}
	return strings.Join(parts, "")
}
