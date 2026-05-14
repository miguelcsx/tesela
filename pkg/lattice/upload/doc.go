// Package upload owns the dataset ingestion lifecycle:
//
//	pending → uploaded → discovering → ready_for_mapping →
//	mapping_confirmed → validating → loading →
//	validating_post → committing → completed (or failed)
//
// Manager.Initiate creates an Upload row and a signed PUT URL the client
// uses to stream a file directly to object storage. Subsequent stages
// (discovery, mapping, validation, bulk load) run from the worker.
package upload
