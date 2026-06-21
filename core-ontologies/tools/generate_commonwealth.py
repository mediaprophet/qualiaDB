#!/usr/bin/env python3
"""Deterministic generator for the Commonwealth Charter values-credential N3.

Source: https://thecommonwealth.org/charter  (Drupal accordion; section HEADINGS
are NOT in the plain-text export — captured here verbatim from the page HTML).
Adopted by Heads of Government 2012; signed by HM Queen Elizabeth II 11 Mar 2013.

NOT a UN/OHCHR instrument -> written to core-ontologies/regional/.
Deontic typing = conservative lexical heuristic, tagged values:HeuristicDerived
(jurisprudential review pending), same convention as generate_n3_from_ohchr.py.
Run:  python3 core-ontologies/tools/generate_commonwealth.py
"""
import re, os

OUT = "core-ontologies/regional"
SRC = "https://thecommonwealth.org/charter"
SLUG = "commonwealth-charter"
TITLE = "Commonwealth Charter"
DATE = "2013"

# Preamble recitals ("We the people of the Commonwealth") — verbatim.
PREAMBLE = (
    "We the people of the Commonwealth. "
    "Recognising that in an era of changing economic circumstances and uncertainty, new trade and economic "
    "patterns, unprecedented threats to peace and security, and a surge in popular demands for democracy, "
    "human rights and broadened economic opportunities, the potential of and need for the Commonwealth - as a "
    "compelling force for good and as an effective network for co-operation and for promoting development - has "
    "never been greater. "
    "Recalling that the Commonwealth is a voluntary association of independent and equal sovereign states, each "
    "responsible for its own policies, consulting and co-operating in the common interests of our peoples and "
    "in the promotion of international understanding and world peace, and influencing international society to "
    "the benefit of all through the pursuit of common principles and values. "
    "Affirming that the special strength of the Commonwealth lies in the combination of our diversity and our "
    "shared inheritance in language, culture and the rule of law; and bound together by shared history and "
    "tradition; by respect for all states and peoples; by shared values and principles and by concern for the "
    "vulnerable. "
    "Affirming that the Commonwealth way is to seek consensus through consultation and the sharing of "
    "experience, especially through practical co-operation, and further affirming that the Commonwealth is "
    "uniquely placed to serve as a model and as a catalyst for new forms of friendship and co-operation in the "
    "spirit of the Charter of the United Nations. "
    "Affirming the role of the Commonwealth as a recognised intergovernmental champion of small states, "
    "advocating for their special needs; providing policy advice on political, economic and social development "
    "issues; and delivering technical assistance. "
    "Welcoming the valuable contribution of the network of the many intergovernmental, parliamentary, "
    "professional and civil society bodies which support the Commonwealth and which subscribe and adhere to its "
    "values and principles. "
    "Affirming the validity of and our commitment to the values and principles of the Commonwealth as defined "
    "and strengthened over the years including: the Singapore Declaration of Commonwealth Principles, the "
    "Harare Commonwealth Declaration, the Langkawi Declaration on the Environment, the Millbrook Action "
    "Programme, the Latimer House Principles, the Aberdeen Agenda, the Trinidad and Tobago Affirmation of "
    "Commonwealth Values and Principles, the Munyonyo Statement on Respect and Understanding, the Lake Victoria "
    "Commonwealth Climate Change Action Plan, the Perth Declaration on Food Security Principles, and the "
    "Commonwealth Declaration on Investing in Young People. "
    "Affirming our core Commonwealth principles of consensus and common action, mutual respect, inclusiveness, "
    "transparency, accountability, legitimacy, and responsiveness. "
    "Reaffirming the core values and principles of the Commonwealth as declared by this Charter."
)

# (number, heading, verbatim body) — headings recovered from page HTML accordions.
SECTIONS = [
    (1, "Democracy",
     "We recognise the inalienable right of individuals to participate in democratic processes, in particular "
     "through free and fair elections in shaping the society in which they live. Governments, political parties "
     "and civil society are responsible for upholding and promoting democratic culture and practices and are "
     "accountable to the public in this regard. Parliaments and representative local governments and other "
     "forms of local governance are essential elements in the exercise of democratic governance. "
     "We support the role of the Commonwealth Ministerial Action Group to address promptly and effectively all "
     "instances of serious or persistent violations of Commonwealth values without any fear or favour."),
    (2, "Human rights",
     "We are committed to the Universal Declaration of Human Rights and other relevant human rights covenants "
     "and international instruments. We are committed to equality and respect for the protection and promotion "
     "of civil, political, economic, social and cultural rights, including the right to development, for all "
     "without discrimination on any grounds as the foundations of peaceful, just and stable societies. "
     "We note that these rights are universal, indivisible, interdependent and interrelated and cannot be "
     "implemented selectively. We are implacably opposed to all forms of discrimination, whether rooted in "
     "gender, race, colour, creed, political belief or other grounds."),
    (3, "International peace and security",
     "We firmly believe that international peace and security, sustainable economic growth and development and "
     "the rule of law are essential to the progress and prosperity of all. We are committed to an effective "
     "multilateral system based on inclusiveness, equity, justice and international law as the best foundation "
     "for achieving consensus and progress on major global challenges including piracy and terrorism. "
     "We support international efforts for peace and disarmament at the United Nations and other multilateral "
     "institutions. We will contribute to the promotion of international consensus on major global political, "
     "economic and social issues. We will be guided by our commitment to the security, development and "
     "prosperity of every member state. "
     "We reiterate our absolute condemnation of all acts of terrorism in whatever form or wherever they occur "
     "or by whomsoever perpetrated, with the consequent tragic loss of human life and severe damage to "
     "political, economic and social stability. We reaffirm our commitment to work together as a diverse "
     "community of nations, individually, and collectively under the auspices and authority of the United "
     "Nations, to take concerted and resolute action to eradicate terrorism."),
    (4, "Tolerance, respect and understanding",
     "We emphasise the need to promote tolerance, respect, understanding, moderation and religious freedom "
     "which are essential to the development of free and democratic societies, and recall that respect for the "
     "dignity of all human beings is critical to promoting peace and prosperity. "
     "We accept that diversity and understanding the richness of our multiple identities are fundamental to the "
     "Commonwealth's principles and approach."),
    (5, "Freedom of Expression",
     "We are committed to peaceful, open dialogue and the free flow of information, including through a free and "
     "responsible media, and to enhancing democratic traditions and strengthening democratic processes."),
    (6, "Separation of Powers",
     "We recognise the importance of maintaining the integrity of the roles of the Legislature, Executive and "
     "Judiciary. These are the guarantors in their respective spheres of the rule of law, the promotion and "
     "protection of fundamental human rights and adherence to good governance."),
    (7, "Rule of Law",
     "We believe in the rule of law as an essential protection for the people of the Commonwealth and as an "
     "assurance of limited and accountable government. In particular we support an independent, impartial, "
     "honest and competent judiciary and recognise that an independent, effective and competent legal system is "
     "integral to upholding the rule of law, engendering public confidence and dispensing justice."),
    (8, "Good Governance",
     "We reiterate our commitment to promote good governance through the rule of law, to ensure transparency "
     "and accountability and to root out, both at national and international levels, systemic and systematic "
     "corruption."),
    (9, "Sustainable Development",
     "We recognise that sustainable development can help to eradicate poverty by pursuing inclusive growth "
     "whilst preserving and conserving natural ecosystems and promoting social equity. "
     "We stress the importance of sustainable economic and social transformation to eliminate poverty and meet "
     "the basic needs of the vast majority of the people of the world and reiterate that economic and social "
     "progress enhances the sustainability of democracy. "
     "We are committed to removing wide disparities and unequal living standards as guided by internationally "
     "agreed development goals. We are also committed to building economic resilience and promoting social "
     "equity, and we reiterate the value in technical assistance, capacity building and practical cooperation "
     "in promoting development. "
     "We are committed to an effective, equitable, rules-based multilateral trading system, the freest possible "
     "flow of multilateral trade on terms fair and equitable to all, while taking into account the special "
     "requirements of small states and developing countries. "
     "We also recognise the importance of information and communication technologies as powerful instruments of "
     "development; delivering savings, efficiencies and growth in our economies, as well as promoting "
     "education, learning and the sharing of culture. We are committed to strengthening its use while enhancing "
     "its security, for the purpose of advancing our societies."),
    (10, "Protecting the Environment",
     "We recognise the importance of the protection and conservation of our natural ecosystems and affirm that "
     "sustainable management of the natural environment is the key to sustained human development. We recognise "
     "the importance of multilateral cooperation, sustained commitment and collective action, in particular by "
     "addressing the adaptation and mitigation challenges of climate change and facilitating the development, "
     "diffusion and deployment of affordable environmentally friendly technologies and renewable energy, and "
     "the prevention of illicit dumping of toxic and hazardous waste as well as the prevention and mitigation "
     "of erosion and desertification."),
    (11, "Access to Health, Education, Food and Shelter",
     "We recognise the necessity of access to affordable health care, education, clean drinking water, "
     "sanitation and housing for all citizens and emphasise the importance of promoting health and well-being "
     "in combating communicable and non-communicable diseases. "
     "We recognise the right of everyone to have access to safe, sufficient and nutritious food, consistent "
     "with the progressive realisation of the right to adequate food in the context of national food security."),
    (12, "Gender Equality",
     "We recognise that gender equality and women's empowerment are essential components of human development "
     "and basic human rights. The advancement of women's rights and the education of girls are critical "
     "preconditions for effective and sustainable development."),
    (13, "Importance of Young People in the Commonwealth",
     "We recognise the positive and active role and contributions of young people in promoting development, "
     "peace, democracy and in protecting and promoting other Commonwealth values, such as tolerance and "
     "understanding, including respect for other cultures. The future success of the Commonwealth rests with "
     "the continued commitment and contributions of young people in promoting and sustaining the Commonwealth "
     "and its values and principles, and we commit to investing in and promoting their development, "
     "particularly through the creation of opportunities for youth employment and entrepreneurship."),
    (14, "Recognition of the Needs of Small States",
     "We are committed to assisting small and developing states in the Commonwealth, including the particular "
     "needs of small island developing states, in tackling their particular economic, energy, climate change "
     "and security challenges, and in building their resilience for the future."),
    (15, "Recognition of the Needs of Vulnerable States",
     "We are committed to collaborating to find ways to provide immediate help to the poorest and most "
     "vulnerable including least developed countries, and to develop responses to protect the people most at "
     "risk."),
    (16, "The Role of Civil Society",
     "We recognise the important role that civil society plays in our communities and countries as partners in "
     "promoting and supporting Commonwealth values and principles, including the freedom of association and "
     "peaceful assembly, and in achieving development goals."),
]

CLOSING = (
    "We are committed to ensuring that the Commonwealth is an effective association, responsive to members' "
    "needs, and capable of addressing the significant global challenges of the future. "
    "We aspire to a Commonwealth that is a strong and respected voice in the world, speaking out on major "
    "issues; that strengthens and enlarges its networks; that has a global relevance and profile; and that is "
    "devoted to improving the lives of all peoples of the Commonwealth."
)


def esc(t):
    t = t.replace("\\", "\\\\").replace('"', '\\"')
    return re.sub(r"\s+", " ", t).strip()


def deontic(t):
    tl = t.lower()
    if "has the right" in tl or "have the right" in tl or "the right of" in tl \
       or "inalienable right" in tl or "entitled to" in tl:
        return ("values:Right", "values:heldBy values:NaturalPerson")
    if "we are committed" in tl or "we commit" in tl or "we will" in tl \
       or "we support" in tl or "we reaffirm" in tl or "we reiterate" in tl:
        return ("values:Obligation", "values:borneBy values:Agent")
    return ("values:Undertaking", "values:borneBy values:Agent")


def main():
    os.makedirs(OUT, exist_ok=True)
    ns = f"https://ns.webcivics.org/values/{SLUG}#"
    L = [
        "@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .",
        "@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .",
        "@prefix dc:     <http://purl.org/dc/terms/> .",
        "@prefix values: <https://ns.webcivics.org/values/> .",
        f"@prefix doc:    <{ns}> .",
        "",
        "# ============================================================",
        f"# {TITLE}",
        "# Regional intergovernmental values charter (NON-UN).",
        "# Section headings recovered from page HTML (hidden from text export).",
        "# Deontic typing = conservative lexical heuristic, tagged",
        "# values:HeuristicDerived - PENDING jurisprudential review.",
        "# ============================================================",
        "",
        "doc:Instrument a values:ValuesCredential ;",
        f'    dc:title "{esc(TITLE)}" ;',
        f'    dc:date "{DATE}" ;',
        f"    values:source <{SRC}> ;",
        f'    values:category "Regional & intergovernmental charters" ;',
        f"    values:categoryStatus values:AutoAssigned ;",
        '    rdfs:comment "Affirmable values credential; deontic layer pending review." .',
        "",
    ]
    # preamble
    dt, bearer = deontic(PREAMBLE)
    L += [
        "doc:preamble a values:Undertaking ;",
        "    values:partOf doc:Instrument ;",
        "    values:borneBy values:Agent ;",
        '    dc:title "Preamble" ;',
        "    values:deonticStatus values:HeuristicDerived ;",
        f'    values:originalText "{esc(PREAMBLE)}" .',
        "",
    ]
    for num, head, body in SECTIONS:
        dt, bearer = deontic(body)
        L += [
            f"doc:section-{num} a {dt} ;",
            "    values:partOf doc:Instrument ;",
            f"    {bearer} ;",
            f'    dc:title "{esc(str(num) + ". " + head)}" ;',
            "    values:deonticStatus values:HeuristicDerived ;",
            f'    values:originalText "{esc(body)}" .',
            "",
        ]
    L += [
        "doc:closing a values:Undertaking ;",
        "    values:partOf doc:Instrument ;",
        "    values:borneBy values:Agent ;",
        '    dc:title "Closing aspirations" ;',
        "    values:deonticStatus values:HeuristicDerived ;",
        f'    values:originalText "{esc(CLOSING)}" .',
        "",
    ]
    path = os.path.join(OUT, f"{SLUG}.n3")
    open(path, "w", encoding="utf-8").write("\n".join(L))
    print(f"  {len(SECTIONS)+2} provisions -> {path}")


if __name__ == "__main__":
    main()
