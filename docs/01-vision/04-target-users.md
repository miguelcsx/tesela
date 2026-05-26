# Target Users

## Primary: Platform Engineering Teams

Teams responsible for building and maintaining internal operational platforms. They have strong backend engineering skills, operate on cloud infrastructure, and face the repeated problem of building domain APIs, access control, and audit infrastructure from scratch. Tesela eliminates this recurring work.

Characteristics: familiar with Go, TypeScript, or Python; operate Kubernetes or managed container services; use Postgres, BigQuery, or equivalent; need to serve multiple internal product teams from a shared operational platform.

## Secondary: Data Platform Teams

Teams that own the data layer and need to expose curated operational data to internal consumers — analysts, product managers, and AI agents — in a governed, typed, queryable form. Tesela bridges the gap between the data warehouse and the operational application layer.

Characteristics: familiar with dbt, Airflow, or Dagster; operate BigQuery, Snowflake, or Redshift; need to expose data assets as typed objects to non-engineering consumers.

## Tertiary: AI/Agent Engineering Teams

Teams building AI-assisted operational workflows. They need governed, typed, audited interfaces through which agents can query data and trigger actions. Tesela provides the agent tool layer automatically from the ontology, with policy enforcement matching that of human users.

Characteristics: familiar with LLM APIs; building workflows where agents need to access operational data and trigger domain actions; require audit trails for all agent operations.

## Domain Examples

Each of the following represents a distinct ontology definition but the same Tesela runtime:

- **CRM platform**: Customer, Deal, Contact, Activity, Campaign
- **Healthcare operations**: Patient, Encounter, Medication, Provider, Claim, Authorization
- **Logistics**: Shipment, Route, Vehicle, Warehouse, Order, Carrier
- **Urban planning**: Scenario, Trip, Zone, Facility, Annotation, Model
- **Financial risk**: Account, Trade, Portfolio, Position, Risk Limit, Alert
- **Supply chain**: Supplier, Purchase Order, SKU, Inventory, Facility, Inspection
- **HR platform**: Employee, Department, Role, Performance Review, Compensation Band

## What Users Are Not

Tesela is not aimed at solo developers building personal projects. The overhead of defining an ontology and deploying a runtime is appropriate for teams building shared operational infrastructure, not for individuals building small applications.
