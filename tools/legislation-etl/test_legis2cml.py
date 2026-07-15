import unittest

import legis2cml as etl


class SegmentationTests(unittest.TestCase):
    def test_legislation_html_has_distinct_legal_information_notice(self):
        inst = etl.Instrument("Example Instrument 2025", "example-instrument-2025", "AU",
                              "https://example.test/instrument", None, [])
        rendered = etl.render_html(inst, {}, "example.pdf", "a" * 64,
                                   register_id="F2025C00572")
        self.assertIn("Legal information, not legal advice", rendered)
        self.assertIn("https://www.legislation.gov.au/F2025C00572", rendered)
        self.assertIn("official or authorised version of Australian law", rendered)
        self.assertIn("CC BY 4.0", rendered)
        self.assertIn("CC BY-NC-ND 4.0", rendered)
        self.assertIn("Copyright &copy; 2026", rendered)
        self.assertIn('data-rights-scope="source-legislation"', rendered)
        self.assertIn('data-rights-scope="technical-work"', rendered)
        self.assertIn('href="/ai-use-policy.json"', rendered)
        self.assertIn("Automated retrieval, indexing, semantic parsing, grounding", rendered)
        self.assertIn("/legal-information", rendered)

    def test_unrecognised_register_id_uses_register_home(self):
        self.assertEqual(etl.federal_register_url("not-an-id"),
                         "https://www.legislation.gov.au/")

    def test_eu_legislation_uses_eli_and_eur_lex_rights(self):
        eli = "http://data.europa.eu/eli/reg/2016/679/oj"
        inst = etl.Instrument("GDPR", "32016r0679", "EU",
                              "https://example.test/gdpr", eli, [])
        rendered = etl.render_html(inst, {}, "gdpr.pdf", "b" * 64)
        self.assertIn(eli, rendered)
        self.assertIn("official or authorised version of European Union law", rendered)
        self.assertIn("EUR-Lex permits reuse of legal documents", rendered)
        self.assertIn("published in the Official Journal", rendered)
        self.assertNotIn("Australian law", rendered)

    def test_register_links_preserve_title_vs_compilation_semantics(self):
        self.assertEqual(
            etl.federal_register_url("C2009A00070"),
            "https://www.legislation.gov.au/C2009A00070/latest/downloads",
        )
        self.assertEqual(
            etl.federal_register_url("C2026C00206VOL04"),
            "https://www.legislation.gov.au/C2026C00206",
        )
        self.assertEqual(
            etl.federal_register_url("F2025C00572"),
            "https://www.legislation.gov.au/F2025C00572",
        )
        self.assertEqual(etl.federal_register_id("C2021A00003REC01"), "C2021A00003")

    def test_full_logic_suite_is_registered(self):
        self.assertEqual(set(etl.LOGIC_SUITE), {
            "Deontic", "Epistemic", "LTL", "Paraconsistent", "ASP", "Dialectical",
            "LinearLogic", "DescriptionLogic", "Argumentation", "AllenInterval",
            "Diffusion", "CogAI", "SHACL", "N3Logic",
        })

    def test_multiline_title_and_duplicate_numbers_get_unique_fragments(self):
        pages = [
            (1, "Australian Federal Police Legislation\nAmendment Act 2000\nNo. 9, 2000"),
            (2, "The Parliament of Australia enacts:\n1  Short title\nText\n1  Repeated item\nMore"),
        ]
        title, provisions = etl.parse_pages(pages, None)
        sections = [provision for provision in provisions if provision.kind == "section"]
        self.assertEqual(title, "Australian Federal Police Legislation Amendment Act 2000")
        self.assertEqual([provision.frag for provision in sections], ["sec-1", "sec-1-2"])

    def test_enacting_formula_skips_contents_front_matter(self):
        pages = [
            (1, "Example Amendment Act 2004\nNo. 5, 2004"),
            (2, "Contents\n1  Short title\n2  Commencement\nPart 2—Widgets\n3  Widget duty\n"
                "The Parliament of Australia enacts:\n"
                "1  Short title\nThis Act may be cited as the Example Amendment Act 2004.\n"
                "2  Commencement\nThis Act commences on Royal Assent.\n"),
        ]
        title, provisions = etl.parse_pages(pages, "Example Amendment Act 2004")
        sections = [p for p in provisions if p.kind == "section"]
        # Only the two body sections after 'enacts' — the Contents arrangement is skipped,
        # so there is no phantom Part and no sec-1-2 collision from the duplicated numbers.
        self.assertEqual([p.frag for p in sections], ["sec-1", "sec-2"])
        self.assertTrue(all(p.text for p in sections))
        self.assertEqual([p.frag for p in provisions if p.kind == "part"], [])

    def test_eu_numbered_article_paragraphs_are_not_sections(self):
        pages = [(1,
            "Whereas:\n(1) A recital.\nCHAPTER I\nGeneral provisions\n"
            "Article 1\nSubject-matter and objectives\n"
            "1. This Regulation lays down rules.\n2. It protects persons.\n"
            "Article 2\nMaterial scope\n1. This Regulation applies.\n")]
        _title, provisions = etl.parse_pages(pages, "GDPR")
        self.assertEqual([p.frag for p in provisions],
                         ["chapter-i", "article-1", "article-2"])
        self.assertIn("1. This Regulation lays down rules.", provisions[1].text)
        self.assertIn("2. It protects persons.", provisions[1].text)

    def test_schedule_items_are_namespaced_and_do_not_collide(self):
        pages = [(1,
            "The Parliament of Australia enacts:\n"
            "1  Short title\nThis Act may be cited as the Example Amendment Act 2004.\n"
            "2  Commencement\nThis Act commences on Royal Assent.\n"
            "Schedule 1—Amendment of the Example Act\n"
            "1  After section 3\nInsert:\nnew material\n"
            "2  Subsection 4(1)\nRepeal the definition.\n")]
        title, provisions = etl.parse_pages(pages, "Example Amendment Act 2004")
        frags = [p.frag for p in provisions]
        kinds = {p.frag: p.kind for p in provisions}
        self.assertIn("sec-1", frags)          # principal sections keep bare ids
        self.assertIn("sec-2", frags)
        self.assertIn("sch-1-sec-1", frags)    # schedule items namespaced to the schedule
        self.assertIn("sch-1-sec-2", frags)
        self.assertNotIn("sec-1-2", frags)     # the old collision is gone
        self.assertEqual(kinds.get("sch-1"), "schedule")

    def test_prose_reference_is_not_a_structural_heading(self):
        pages = [(1,
            "The Parliament of Australia enacts:\n"
            "3A  Overview of Act\n"
            "Division 2 of Part III covers matters to do with the employment of staff.\n"
            "Part XI of the Crimes Act 1914 continues to apply.\n"
            "4  Real section\nBody text.\n")]
        title, provisions = etl.parse_pages(pages, "Example Act 2004")
        # The 'Division 2 of…' / 'Part XI of…' cross-references are prose, not structural nodes.
        self.assertEqual([p.frag for p in provisions if p.kind in ("part", "division")], [])
        overview = next(p for p in provisions if p.frag == "sec-3a")
        self.assertIn("Division 2 of Part III", overview.text)
        self.assertEqual([p.frag for p in provisions if p.kind == "section"], ["sec-3a", "sec-4"])

    def test_repeated_schedule_header_is_not_duplicated(self):
        pages = [(1,
            "The Parliament of Australia enacts:\n"
            "Schedule 1—Amendment of the Example Act\n"
            "1  After section 3\nInsert:\ntext one\n"
            "Schedule 1—Amendment of the Example Act\n"  # running page header, same schedule
            "text two\n"
            "2  Subsection 4(1)\nRepeal.\n")]
        title, provisions = etl.parse_pages(pages, "Example Amendment Act 2004")
        self.assertEqual(len([p for p in provisions if p.kind == "schedule"]), 1)
        item1 = next(p for p in provisions if p.frag == "sch-1-sec-1")
        self.assertIn("text one", item1.text)
        self.assertIn("text two", item1.text)  # not flushed by the repeated running header

    def test_title_inference_robust_on_cover_variants(self):
        # year as a page-date ABOVE the title (old scan) -> stripped from the front
        self.assertEqual(
            etl.infer_title([(1, "1955.\nAustralian Capital Territory and Jervis Bay\n"
                                 "(Lands Acquisition).\nNo. 70 of 1955.\nAn Act relating to land.")]),
            "Australian Capital Territory and Jervis Bay (Lands Acquisition)")
        # year BETWEEN the title and the Act number -> kept as the title's own year
        self.assertEqual(
            etl.infer_title([(1, "Crimes Legislation Amendment Act (No. 2)\n1989\n"
                                 "No. 4 of 1990\nTABLE OF PROVISIONS")]),
            "Crimes Legislation Amendment Act (No. 2) 1989")
        # a SCALEplus/register URL above the title must not be swept in
        self.assertEqual(
            etl.infer_title([(1, "Note: available in SCALEplus\n(http://scaleplus.law.gov.au/x)\n"
                                 "Australian Heritage Council\n(Consequential) Act 2003\nNo. 86, 2003")]),
            "Australian Heritage Council (Consequential) Act 2003")
        # a numbered body clause must never be mistaken for the title
        self.assertNotIn("shall come into operation",
                         etl.infer_title([(1, "2. This Act shall come into operation on a day. "
                                              "3. The Seal Act 1908 is repealed.")]) or "")

    def test_historical_inline_sections_and_citation_title(self):
        pages = [(1, """CRIMES.
No. 6 of 1915.
An Act to amend the Crimes Act 1914.
1.—(1.) This Act may be cited as the Crimes Act 1915.
(2.) It continues during the war.
2. Section eighty-six is amended.
3. This Act is deemed to have commenced earlier.
""")]
        title, provisions = etl.parse_pages(pages, None)
        sections = [provision for provision in provisions if provision.kind == "section"]
        self.assertEqual(title, "Crimes Act 1915")
        self.assertEqual([provision.number for provision in sections], ["1", "2", "3"])
        self.assertIn("may be cited", sections[0].text)

    def test_eu_regulation_chapters_and_articles_skip_recitals(self):
        pages = [(1, """REGULATION (EU) 2016/679
Whereas:
(1) Personal-data protection is a fundamental right.
CHAPTER I
General provisions
Article 1
Subject-matter and objectives
1. This Regulation lays down rules relating to personal data.
Article 2
Material scope
1. This Regulation applies to automated processing.
""")]
        title, provisions = etl.parse_pages(pages, "General Data Protection Regulation")
        self.assertEqual(title, "General Data Protection Regulation")
        self.assertEqual([p.frag for p in provisions],
                         ["chapter-i", "article-1", "article-2"])
        self.assertEqual([p.heading for p in provisions],
                         ["General provisions", "Subject-matter and objectives", "Material scope"])
        self.assertNotIn("fundamental right", " ".join(p.text for p in provisions))
        self.assertIn("lays down rules", provisions[1].text)

    def test_long_provision_is_bounded_and_overlapped(self):
        text = "\n\n".join(f"Clause {index}. " + ("x" * 180) for index in range(18))
        chunks = etl.split_text(text, 1000, 120)
        self.assertGreater(len(chunks), 1)
        self.assertTrue(all(len(chunk) <= 1000 for chunk in chunks))
        for left, right in zip(chunks, chunks[1:]):
            self.assertTrue(any(left.endswith(right[:size]) for size in range(1, 121)))

    def test_segments_are_content_addressed_and_use_fragment_keys(self):
        provisions = [
            etl.Provision("sec-1", "section", "1", "Duty", "A person must act.", 2),
            etl.Provision("sec-2", "section", "2", "Right", "A person may apply.", 3),
        ]
        first = etl.build_segments(provisions, 1000, 100)
        second = etl.build_segments(provisions, 1000, 100)
        self.assertEqual([item.segment_id for item in first], [item.segment_id for item in second])
        self.assertEqual({item["key"] for segment in first for item in segment.items},
                         {"sec-1", "sec-2"})

    def test_invalid_model_shape_is_rejected(self):
        self.assertIsNone(etl._normalise_classification({"deonticType": "Guess"}))
        self.assertIsNone(etl._normalise_classification(
            {"deonticType": "Right", "conditions": "not-a-list"}
        ))
        normalised = etl._normalise_classification(
            {"deonticType": "Undertaking", "mustProvide": False, "borneBy": "Unknown"}
        )
        self.assertNotIn("mustProvide", normalised)
        self.assertNotIn("borneBy", normalised)
        valid = etl._normalise_classification({
            "deonticType": "Undertaking",
            "logicApplications": [{"logic": "LTL", "operator": "Globally",
                                   "summary": "The condition persists.", "confidence": 0.8}],
        })
        self.assertEqual(valid["logicApplications"][0]["logic"], "LTL")
        low_confidence = etl._normalise_classification({
            "deonticType": "Undertaking",
            "logicApplications": [{"logic": "DescriptionLogic", "operator": "Applicable",
                                   "summary": "Weak guess", "confidence": 0.1}],
        })
        self.assertEqual(low_confidence["logicApplications"], [])
        deontic = etl._normalise_classification({
            "deonticType": "Obligation", "summary": "Must report", "confidence": 0.9,
            "logicApplications": [],
        })
        self.assertEqual(deontic["logicApplications"][0]["logic"], "Deontic")
        self.assertEqual(deontic["logicApplications"][0]["operator"], "Obligate")
        reconciled = etl._normalise_classification({
            "deonticType": "Obligation", "summary": "No fraud", "confidence": 0.9,
            "logicApplications": [{"logic": "Deontic", "operator": "Prohibition",
                                   "summary": "Fraud is forbidden", "confidence": 0.9}],
        })
        self.assertEqual(reconciled["deonticType"], "Prohibition")
        self.assertIsNone(etl._normalise_classification({
            "deonticType": "Undertaking",
            "logicApplications": [{"logic": "InventedLogic", "summary": "No."}],
        }))

    def test_contentless_logic_application_is_dropped_not_fatal(self):
        # Regression: amendment-machinery fragments ("Insert:" / "Add:") make the 3B model emit
        # valid-but-empty, confidence-0.0 applications. These must be dropped while keeping the
        # provision's Undertaking classification — not reject the whole provision, which left such
        # segments permanently pending and failed the file with rc=2 on every --resume.
        normalised = etl._normalise_classification({
            "deonticType": "Undertaking", "borneBy": None, "heldBy": None,
            "summary": "", "mustProvide": "", "conditions": [], "crossReferences": [],
            "confidence": 0.0,
            "logicApplications": [
                {"logic": "DescriptionLogic", "operator": "", "summary": "",
                 "premise": "", "conclusion": "", "confidence": 0.0},
                {"logic": "AllenInterval", "operator": "", "summary": "",
                 "premise": "", "conclusion": "", "confidence": 0.0},
            ],
        })
        self.assertIsNotNone(normalised)
        self.assertEqual(normalised["deonticType"], "Undertaking")
        self.assertEqual(normalised["logicApplications"], [])
        # A valid, contentful application alongside a contentless one survives; the empty one is dropped.
        mixed = etl._normalise_classification({
            "deonticType": "Undertaking", "confidence": 0.9,
            "logicApplications": [
                {"logic": "LTL", "operator": "Globally", "summary": "Recurs annually.",
                 "premise": "", "conclusion": "", "confidence": 0.8},
                {"logic": "AllenInterval", "operator": "", "summary": "",
                 "premise": "", "conclusion": "", "confidence": 0.0},
            ],
        })
        self.assertEqual([a["logic"] for a in mixed["logicApplications"]], ["LTL"])

    def test_num_ctx_covers_segment_budget_and_floors_at_8192(self):
        # Ollama's default window is only 2048 and truncates longer prompts; the chosen window
        # must cover the largest prompt + output. Measured worst case (8000-char excerpt) is a
        # ~2460-token prompt + 1800-token answer = ~4260; 8192 is the floor and covers it.
        self.assertEqual(etl.choose_num_ctx(8000, 1), 8192)
        self.assertGreaterEqual(etl.choose_num_ctx(8000, 1), 2460 + etl.NUM_PREDICT)
        # a larger segment budget scales the window up, always to a power of two
        larger = etl.choose_num_ctx(32000, 3)
        self.assertGreater(larger, 8192)
        self.assertEqual(larger & (larger - 1), 0)

    def test_split_subsections_produces_points_with_parent(self):
        sec = etl.Provision("sec-2", "section", "2", "Commencement",
                            "This section provides:\n(1) Part A commences on assent.\n"
                            "(2) Part B by proclamation.\n(3) Otherwise after 6 months.", 1)
        subs = etl.split_subsections(sec)
        self.assertEqual([s.frag for s in subs], ["sec-2-ss-1", "sec-2-ss-2", "sec-2-ss-3"])
        self.assertTrue(all(s.parent == "sec-2" for s in subs))
        self.assertEqual([s.number for s in subs], ["2(1)", "2(2)", "2(3)"])
        self.assertEqual(sec.text, "This section provides:")  # lead-in kept on the section
        flat = etl.Provision("sec-3", "section", "3", "Simple", "Just one paragraph.", 1)
        self.assertEqual(etl.split_subsections(flat), [])  # <2 subsections: not split

    def test_classifiable_and_concept_units_partition(self):
        provs = [
            etl.Provision("sec-1", "section", "1", "Leaf", "no subsections here", 1),
            etl.Provision("sec-2", "section", "2", "Container", "lead-in", 1),
            etl.Provision("sec-2-ss-1", "subsection", "2(1)", "Container", "(1) x", 1, parent="sec-2"),
            etl.Provision("sec-2-ss-2", "subsection", "2(2)", "Container", "(2) y", 1, parent="sec-2"),
        ]
        # classifiable = leaf sections + subsections (the container itself is not classified)
        self.assertEqual([p.frag for p in etl.classifiable_units(provs)],
                         ["sec-1", "sec-2-ss-1", "sec-2-ss-2"])
        # concept units = every section + subsection (containers become structural concepts)
        self.assertEqual([p.frag for p in etl.concept_units(provs)],
                         ["sec-1", "sec-2", "sec-2-ss-1", "sec-2-ss-2"])

    def test_subsection_hierarchy_and_metadata_in_graph(self):
        sec = etl.Provision("sec-5", "section", "5", "Duties", "The following apply:", 2)
        ss1 = etl.Provision("sec-5-ss-1", "subsection", "5(1)", "Duties",
                            "(1) A person must report.", 2, "h1", parent="sec-5")
        ss2 = etl.Provision("sec-5-ss-2", "subsection", "5(2)", "Duties",
                            "(2) A person may appeal.", 2, "h2", parent="sec-5")
        inst = etl.Instrument("Example Act 2020", "example-act-2020", "AU", "https://ex/act",
                              None, [sec, ss1, ss2], act_no="7", year="2020",
                              long_title="An Act to do things.")
        cls = {"sec-5-ss-1": {"deonticType": "Obligation", "confidence": 0.9,
                              "logicApplications": [{"logic": "Deontic", "operator": "Obligate",
                                                     "summary": "duty", "premise": "", "conclusion": "",
                                                     "confidence": 0.9}]},
               "sec-5-ss-2": {"deonticType": "Permission", "confidence": 0.8, "logicApplications": []}}
        n3 = etl.build_n3(inst, cls, "s.pdf", "s.pdf", "2")
        jsonld = etl.build_jsonld(inst, cls, "s.pdf")
        # instrument metadata node
        self.assertIn("a cof:Document", n3)
        self.assertIn('dc:identifier "No. 7 of 2020"', n3)
        # container section: hasPart links, no norm of its own
        self.assertIn("cml:hasPart concept:example-act-2020-sec-5-ss-1", n3)
        self.assertNotIn("concept:example-act-2020-sec-5-norm ", n3)
        # subsections: partOf the parent + their own distinct norms (obligation vs permission)
        self.assertIn("values:partOf concept:example-act-2020-sec-5 ", n3)
        self.assertIn("concept:example-act-2020-sec-5-ss-1-norm a values:Obligation", n3)
        self.assertIn("concept:example-act-2020-sec-5-ss-2-norm a values:Permission", n3)
        ids = [node["@id"] for node in jsonld["@graph"]]
        self.assertIn("https://ex/act", ids)  # instrument node in JSON-LD too
        self.assertIn("concept:example-act-2020-sec-5-ss-2", ids)

    def test_multimodal_outputs_share_cml_routing(self):
        provision = etl.Provision("sec-7", "section", "7", "Temporal duty",
                                  "The agency must report annually.", 4, "abc")
        inst = etl.Instrument("Test Act 2026", "test-act-2026", "AU",
                              "https://example.test/act", None, [provision])
        classifications = {"sec-7": {
            "deonticType": "Obligation", "confidence": 0.9,
            "logicApplications": [
                {"logic": "Deontic", "operator": "Obligate", "summary": "Creates a duty.",
                 "premise": "Agency exists", "conclusion": "Report", "confidence": 0.9},
                {"logic": "LTL", "operator": "Globally", "summary": "Recurring duty.",
                 "premise": "Each year", "conclusion": "Report", "confidence": 0.8},
            ],
        }}
        n3 = etl.build_n3(inst, classifications, "source.pdf", "source.pdf", "2")
        jsonld = etl.build_jsonld(inst, classifications, "source.pdf")
        cogai, chunks = etl.build_cogai(inst, classifications)
        shacl = etl.build_logic_shacl(inst)
        self.assertIn("cml:LogicApplication", n3)
        self.assertIn("cml:LTL", n3)
        self.assertEqual(sum(1 for node in jsonld["@graph"]
                             if node.get("@type") == "cml:LogicApplication"), 2)
        self.assertEqual(chunks, 2)
        self.assertIn("logic: LTL", cogai)
        self.assertIn("sh:targetClass cml:LogicApplication", shacl)

    def test_description_logic_requires_source_hierarchy_language(self):
        classification = {"logicApplications": [{
            "logic": "DescriptionLogic", "operator": "Classify", "summary": "Weak guess",
            "premise": "", "conclusion": "", "confidence": 0.9,
        }]}
        filtered = etl.filter_logic_applications_for_source(
            classification, "This Act may be cited as the Example Act."
        )
        self.assertEqual(filtered["logicApplications"], [])
        classification["logicApplications"] = [{
            "logic": "DescriptionLogic", "operator": "Subsumes", "summary": "Definition",
            "premise": "", "conclusion": "", "confidence": 0.9,
        }]
        kept = etl.filter_logic_applications_for_source(
            classification, "employee means a person appointed under section 4"
        )
        self.assertEqual(len(kept["logicApplications"]), 1)

    def test_long_provision_results_merge_deterministically(self):
        completed = {
            "b": {"results": {"sec-9": {"deonticType": "Obligation", "summary": "B",
                                            "conditions": ["c2"], "crossReferences": [],
                                            "confidence": 0.8}}},
            "a": {"results": {"sec-9": {"deonticType": "Obligation", "summary": "A",
                                            "conditions": ["c1"], "crossReferences": [],
                                            "confidence": 0.6}}},
        }
        merged = etl.aggregate_classifications(completed)["sec-9"]
        self.assertEqual(merged["deonticType"], "Obligation")
        self.assertEqual(merged["summary"], "A B")
        self.assertEqual(merged["conditions"], ["c1", "c2"])
        self.assertEqual(merged["confidence"], 0.7)

    def test_classification_lookup_supports_new_and_old_progress(self):
        provision = etl.Provision("sec-4", "section", "4", "Heading")
        self.assertEqual(etl.classification_for(provision, {"sec-4": {"x": 1}}), {"x": 1})
        self.assertEqual(etl.classification_for(provision, {"4": {"x": 2}}), {"x": 2})


if __name__ == "__main__":
    unittest.main()
