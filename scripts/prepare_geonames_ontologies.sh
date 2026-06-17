#!/usr/bin/env bash
# Ingest bundled GeoNames TTL into a .q42 volume for the Ontology Demo.
exec bash "$(dirname "$0")/prepare_bundled_ontologies.sh" geonames