# tpt-io-openscenario

A structural reader and validator for ASAM OpenSCENARIO 1.x scenario files (`.xosc`).

## Spec access and licence

ASAM OpenSCENARIO is an ASAM standard. Its published document is subject to the ASAM
licence terms, so no specification text, schema or example scenario is copied into this
crate. Test files are written for these tests.

## What it reads

- `<FileHeader>`: revision, date, description and author.
- `<ParameterDeclarations>`: names, declared types and default values (`$name` form kept).
- `<CatalogLocations>`: catalog directories, as `"<Catalog>:<path>"`.
- `<RoadNetwork><LogicFile filepath>`: the OpenDRIVE road file.
- `<Entities>`: each `<ScenarioObject>` with its kind (Vehicle, Pedestrian, MiscObject,
  external reference, or other) and category.
- `<Storyboard>`: stories, their acts, and the count of maneuver groups in each act.

## Validation

`OpenScenario::validate` requires OpenSCENARIO revision 1, and unique entity and story
names.

## What it does not do

- It is read-only: there is no writer, so the round-trip checks used by other crates
  in the workspace do not apply.
- Parameters are not substituted, catalogs are not loaded, and triggers, actions and
  conditions below the act level are not modelled.
- It does not run or simulate the scenario.
