# tpt-io-opcua

A read-only reader for OPC UA NodeSet2 XML files (`UANodeSet`), the exchange format for
OPC UA information models.

## Spec access and licence

OPC UA is published by the OPC Foundation. The NodeSet schema is available under the OPC
Foundation's terms. No schema text, standard node set or companion specification is copied
into this crate; the tests use a small node set written for them.

## What it reads

- `NamespaceUris`: the namespace URIs in index order.
- `Aliases`: each alias name and the node id it stands for.
- Every node element: `UAObject`, `UAVariable`, `UAMethod`, `UAObjectType`,
  `UAVariableType`, `UADataType`, `UAReferenceType` and `UAView`. Each carries its
  `NodeId`, `BrowseName`, optional `DisplayName`, optional `DataType`, and its
  `References`. `IsForward` defaults to `true`.

## Validation

`NodeSet::validate` requires unique NodeIds and a non-empty BrowseName on every node.

## What it does not do

- Read only. There is no writer.
- Node values, `ArrayDimensions`, data type definitions and `Extensions` are not decoded.
- Alias names in reference types and targets are kept as written; they are not resolved.
- Reference targets in other namespaces are not checked against the set.
