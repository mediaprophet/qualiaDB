use qualia_core_db::sparql_library::parsers::turtle_star::TurtleStarParser;
use qualia_core_db::rdf_star::RdfStarParser;

fn main() {
    let mut parser = TurtleStarParser::new(0);
    let input = b"<< yago:Helmut_Kohl schema:award yago:Charlemagne_Prize >> schema:startDate "1988-11-01T00:00:00Z"^^xsd:dateTime";
    match parser.parse_triple(input) {
        Ok(t) => println!("Success: {:?}", t),
        Err(e) => println!("Error: {:?}", e),
    }
}
