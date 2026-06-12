# Ecosystem Notes  

This document is intended to provide a summary overview of the various components that exist as may be useful for Human Centric AI related systems builds; including but not limited to, social-web related functionality.  **NOTE: THIS DOCUMENT IS A WORK IN PROGRESS**

## Importance of 'Namespace' support (Semantic Web vs. json, et.al.)

At a basic level, the means to define vocabulary, taxonomy and fundamentally therefore also, software via the use of language - generally means, defining variables in association to the use of specific linguistic terms.  When these structures are defined from a specific 'namespace', or dictionary, the meaning is specified by that means.  Alot of software is developed to operate in an 'internal only' 'global namespace' kind of structure, as is illustrated by software employing traditional RDBMS (relational Database Management System) interfaces.  Whereas, Semantic Web constructs introduced the employment of a DNS associated URI.  What this 'semantic web' method therefore enables, is the means for words that are otherwise not better defined, to have specified meanings, that are in-turn able to be distinguished by 'agents' (particularly important for software agents and/or AI).  


## Human Centric AI Related protocols & works

- WebSockets: https://websockets.spec.whatwg.org// 
- WebRTC: Real-Time Communication in Browsers: https://www.w3.org/TR/webrtc/ 


### RDF
- RDF: https://www.w3.org/RDF/ 
- RDF-star and SPARQL-star: https://w3c.github.io/rdf-star/cg-spec/editors_draft.html 
- RDF-star: https://w3c.github.io/rdf-star/ 

### Sparql 
Sparql is a query language.
- Sparql: https://www.w3.org/TR/sparql11-query/ 
- SPARQL 1.1 Federated Query: https://www.w3.org/TR/sparql11-federated-query/ 
- SPARQL 1.2 Federated Query: https://www.w3.org/TR/sparql12-federated-query/ 
- SPARQL 1.2 Overview: https://w3c.github.io/sparql-concepts/spec/ 
- SPARQL 1.2 Protocol: https://www.w3.org/TR/sparql12-protocol/
- SPARQL 1.2 Query Language: https://www.w3.org/TR/sparql12-query/ 
- SPARQL 1.2 Update: https://w3c.github.io/sparql-update/spec/ 

### WebID
- WebID 1.0 https://www.w3.org/2005/Incubator/webid/spec/identity/
- The Cert Ontology 1.0:  https://www.w3.org/ns/auth/cert#
- WebID-TLS https://www.w3.org/2005/Incubator/webid/spec/tls/
- WebID-RSA: No Spec Found, but https://github.com/deiu/webid-rsa 
- WebID-OIDC: https://github.com/solid/webid-oidc-spec 

### Verifiable Credentials
- Verifiable Credentials: https://www.w3.org/TR/vc-overview/
- Verifiable Credentials Data Model v2.0: https://www.w3.org/TR/vc-data-model-2.0/ 
- Verifiable Credential Data Model v2.1: https://w3c.github.io/vc-data-model/ 
- Verifiable Credential Data Integrity 1.1 https://w3c.github.io/vc-data-integrity/ 
- Verifiable Credentials Vocabulary v2.0 https://www.w3.org/2018/credentials/ 

### Web Of Things
Web Thing Protocol: https://w3c.github.io/web-thing-protocol/
Web of Things (WoT) Thing Description 1.1: https://www.w3.org/TR/wot-thing-description11/
Web of Things (WoT) Discovery: https://www.w3.org/TR/wot-discovery/ 
Web of Things: https://www.w3.org/WoT/ 
Web of Things (WoT) Architecture 1.1: https://w3c.github.io/wot-architecture/

## Ontology Basics
The RDF Data Cube Vocabulary - https://www.w3.org/TR/vocab-data-cube/ 
Shapes Constraint Language (SHACL): https://www.w3.org/TR/shacl/ 
Shape Expressions Language 2.1: https://shex.io/shex-semantics/index.html 
ODRL Version 2.2 Ontology: https://www.w3.org/ns/odrl/2/ 
ODRL Information Model 2.2: https://www.w3.org/TR/odrl-model/
ODRL V2.2 Implementation Best Practices: https://w3c.github.io/odrl/bp/ 

## Social Web Components
Web Access Control: https://www.w3.org/wiki/WebAccessControl 
Web Annotations https://www.w3.org/TR/annotation-model/ 
Web Annotation Protocol https://www.w3.org/TR/annotation-protocol/

Activity Pub:  https://www.w3.org/TR/activitypub/ 
Activity Streams 2.0: https://www.w3.org/TR/activitystreams-core/ 
Linkd Data Notifications (LDN): https://www.w3.org/TR/ldn/
LDP: https://www.w3.org/TR/ldp/  
Web Payments: https://www.w3.org/Payments/WG/ 
Payment Request API: https://www.w3.org/TR/payment-request/ 

WebCredits: https://webcredits.org/ | https://webcredits.github.io/spec/ 

CogAI Chunks and Rules: https://w3c.github.io/cogai/ 

### Notes about DIDs  

When seeking to achieve support for semantic nuance, semantic web efforts have had several challenges.  One is the means to have and maintain **CoolURIs** [1](https://www.w3.org/Provider/Style/URI) [2](https://www.w3.org/TR/cooluris/).  Whilst attempts to improve suports were made via [WebDAV](https://en.wikipedia.org/wiki/WebDAV), it appears this only resulted in limited success.  

Around 2014, as efforts to consider ecosystems options in environments where a range of relatively newly developed Decentralised Ledger Technology Protocols (DLTs) were being developed, particularly including blockchains (ie: bitcoin) but also both DHTs (Distributed Hash Tables) and hybrids, which are fundamental to the operation of internet (ie: DNS) but have a range of alternatives with different properties; the means to figure out how to provide some sort of harmonised solution, given no clear singular 'one stop shop' HTTP alternative, led to support for works that were believed to offer a path towards resolving this issue.  This was in-turn defined as [Decentralized IDentifier (DID)](https://en.wikipedia.org/wiki/Decentralized_identifier).  Whilst parties who became highly involved had already been working on 'Decentralised IDentity', which is illustrated throughout the consequential works in that area, a lesser considered - yet earlier intended purpose - was to provide the means to store important 'linked-data' (RDF) on non-http protocols, as to address both the namespace related, 'commons' needs for CoolURIs.  

Whether and/or how these works now act as to support these purposes, is yet to be better clarified.

**Related Links**
- [Decentralized Identifiers (DIDs) v1.0](https://www.w3.org/TR/did-1.0/)
- [Decentralized Identifiers (DIDs) v1.1](https://www.w3.org/TR/did-1.1/)
- [Decentralized Identifier Extensions](https://www.w3.org/TR/did-extensions/) (SeeAlso: [GitHub Repo](https://github.com/w3c/did-extensions))
- [DID Method Rubric v1.0](https://w3c.github.io/did-rubric/)


**NOTES:**
Privacy Principles: https://www.w3.org/TR/privacy-principles/ 

- https://solid.github.io/chat/
- https://solidproject.org/TR/protocol 
- Web Of Trust http://xmlns.com/wot/0.1/ 


## Broader Notes

As to be addressed further at a later date; The development of Internet and in-turn also WWW, has had a symbiotic relationship.  People often do not understand IP portfolios and the challenges relating to propriatery intellectual property and/or IPR generally, which acts disaffectively in a plurity of manners, differentiated whilst also pervasive, across the socioeconomic 'digitally instigated' ecology.  One of the major areas thought most important is about **ontology**, and whilst WWW works have been focused on HTTP(x) applications, a belief is that these methods may best be developed across internet more broadly.  the implications of these sorts of considerations, are non-trivial, yet also, somewhat instrumental.  