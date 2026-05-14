// Package graphql builds a *graphql.Schema from a *types.Ontology snapshot
// at runtime. The schema exposes search/get/aggregate queries per object
// type and execute mutations per action type. Schemas are rebuilt on
// ontology hot-reload via the cache's Subscribe channel.
package graphql
