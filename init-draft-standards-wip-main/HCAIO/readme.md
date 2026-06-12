Human-Centric AI Ontology (HCAI) Project
Overview
The Human-Centric AI Ontology (HCAI) is an initiative by the W3C Human-Centric AI Community Group to develop a standalone vocabulary that redefines how natural persons and their intrinsic human qualities are represented in AI systems. Designed as an alternative and potential replacement for OWL and FOAF, HCAI addresses the limitations of these frameworks, particularly the inadequacy of owl:Thing and FOAF’s social metadata in capturing complex human factors such as consciousness, emotions, psychological states, sociological roles, and spiritual dimensions like the soul. The ontology also supports integration with tangible entities (e.g., assets, accounts) through owl:Thing-based extensions, ensuring interoperability with existing RDF ecosystems.
This project is an unofficial proposal, intended to spark discussion and collaboration within the W3C Human-Centric AI Community Group to advance ethical, culturally sensitive, and human-focused AI applications.
Motivation
Current ontologies like OWL and FOAF are insufficient for human-centric AI systems:
OWL’s Limitation: The top-level owl:Thing class is overly generic, failing to represent nuanced human attributes such as consciousness, emotions, or beliefs, which are not mere “things.”

FOAF’s Limitation: FOAF focuses on social metadata (e.g., foaf:name, foaf:knows), neglecting deeper psychological, sociological, and spiritual dimensions of humanity.

Need for Human-Centric AI: AI agents and software systems require a robust framework to model human factors like love, resilience, or cultural roles to support ethical and empathetic interactions.

HCAI aims to fill this gap by providing a vocabulary tailored for AI systems that prioritize natural persons and their intrinsic qualities, while allowing connections to tangible entities for practical applications.
Objectives
The HCAI Ontology seeks to:
Model Intrinsic Human Qualities: Represent consciousness, emotions, psychological states, sociological roles, beliefs, and spiritual aspects (e.g., soul) as distinct from owl:Thing.

Support AI Agents: Enable AI systems to reason about human factors in a culturally sensitive and ethical manner.

Ensure Interoperability: Integrate with existing RDF systems by supporting tangible entities (e.g., assets, accounts) as subclasses of owl:Thing.

Replace and Extend OWL/FOAF: Offer a standalone alternative that better serves human-centric AI applications while maintaining compatibility with broader semantic web standards.

Key Features
Core Classes
hcai:NaturalPerson: Represents a human being with intrinsic qualities, distinct from owl:Thing or foaf:Person.

hcai:Consciousness: Captures subjective awareness, self-reflection, and intentionality.

hcai:Soul: Models the metaphysical essence of a person.

hcai:Emotion: Represents emotional states (e.g., love, grief).

hcai:PsychologicalState: Describes mental conditions (e.g., resilience, anxiety).

hcai:SociologicalRole: Defines socially constructed roles (e.g., caregiver, leader).

hcai:Relationship: Represents emotional or social bonds (e.g., friendship, kinship).

hcai:HumanExperience: Captures lived experiences shaping identity (e.g., cultural immersion).

hcai:ValueBelief: Models deeply held beliefs or values (e.g., compassion, justice).

hcai:TangibleEntity: A subclass of owl:Thing for assets, accounts, or other concrete entities.

Core Properties
hcai:hasConsciousness, hcai:hasSoul, hcai:experiencesEmotion, etc.: Link hcai:NaturalPerson to human-centric attributes.

hcai:ownsEntity: Connects persons to tangible entities (e.g., hcai:TangibleEntity).

Descriptive properties like hcai:emotionType, hcai:roleType, etc., for specifying types (e.g., “love,” “parent”).

Axioms
hcai:NaturalPerson is disjoint with owl:Thing and foaf:Person.

Every hcai:NaturalPerson has exactly one hcai:Consciousness and hcai:Soul.

hcai:Relationship is symmetric.

hcai:TangibleEntity extends owl:Thing for compatibility.

Example Use Cases
Human-Centric AI Assistants:
An AI assistant uses HCAI to understand a user’s emotional state (e.g., hcai:Emotion with hcai:emotionType "Grief") and sociological role (e.g., hcai:SociologicalRole with hcai:roleType "Caregiver") to provide empathetic responses.

Example: Supporting a user through loss by recognizing their hcai:PsychologicalState of resilience.

Ethical AI Reasoning:
AI systems model a user’s hcai:ValueBelief (e.g., hcai:beliefType "Justice") to align recommendations with their ethical framework.

Cultural Sensitivity:
HCAI captures hcai:HumanExperience (e.g., hcai:experienceType "Cultural Immersion") to tailor AI interactions to diverse cultural contexts.

Asset Management:
A person’s ownership of tangible entities (e.g., hcai:TangibleEntity with hcai:entityType "Financial Account") is modeled alongside their human attributes, integrating with existing RDF systems.

Sample Ontology Snippet (Turtle)
turtle

@prefix hcai: <http://www.w3.org/ns/hcai#> .
@prefix ex: <http://example.org/> .

ex:Jamal a hcai:NaturalPerson ;
  hcai:hasConsciousness ex:JamalConsciousness ;
  hcai:experiencesEmotion ex:JamalHope ;
  hcai:fulfillsRole ex:JamalMentor ;
  hcai:ownsEntity ex:JamalCar .

ex:JamalConsciousness a hcai:Consciousness ;
  hcai:description "Jamal's self-awareness shaped by mentorship" .

ex:JamalHope a hcai:Emotion ;
  hcai:emotionType "Hope" ;
  hcai:context "Starting a community project" .

ex:JamalMentor a hcai:SociologicalRole ;
  hcai:roleType "Mentor" .

ex:JamalCar a hcai:TangibleEntity , owl:Thing ;
  hcai:entityType "Physical Asset" .

Getting Involved
The HCAI Ontology is an open, community-driven project. We invite contributions from the W3C Human-Centric AI Community Group and beyond to:
Refine the Ontology: Propose new classes, properties, or axioms to better capture human factors.

Test Use Cases: Implement HCAI in AI systems and share feedback.

Discuss Ethical Implications: Ensure the ontology supports ethical and inclusive AI design.

Extend Interoperability: Explore integrations with other semantic web standards.

To contribute:
Join the W3C Human-Centric AI Community Group.

Submit issues, pull requests, or feedback via the GitHub repository.

Participate in discussions on the community group’s mailing list or forums.

Current Status
This is an unofficial draft, published on June 20, 2025, to initiate discussion within the W3C Human-Centric AI Community Group. The ontology is not yet a formal W3C recommendation but aims to evolve through community input.
Acknowledgments
This project builds on the need for AI systems to prioritize human dignity, ethical reasoning, and cultural sensitivity. We thank the W3C Human-Centric AI Community Group for fostering an environment to explore these critical topics.

