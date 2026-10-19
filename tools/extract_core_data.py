#!/usr/bin/env python3
"""One-off: pull the static data tables out of the Dart sp_core into data/*.json (and copy CSVs).
The Dart code is the source of truth; re-run this if its tables change."""
import json, re, shutil, sys
from pathlib import Path

SRC = Path.home() / "Documents/Source/Semi-ProFB"
CORE = SRC / "packages/sp_core/lib/src"
OUT = Path(__file__).resolve().parent.parent / "data"
OUT.mkdir(exist_ok=True)

STR = r"""(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")"""
TOWN_RE = re.compile(
    r"Town\(\s*id:\s*" + STR + r",\s*name:\s*" + STR + r",\s*division:\s*" + STR +
    r",\s*population:\s*(\d+),\s*wealth:\s*(\d+),\s*turnout:\s*(\d+),\s*loyalty:\s*(\d+),\s*culture:\s*(\d+),\s*biz:\s*(\d+),\s*capacity:\s*(\d+),\s*nickname:\s*" + STR +
    r",\s*mascot:\s*" + STR + r",\s*quirk:\s*" + STR + r"\s*\)")

def s(m, i):  # pick whichever quote group matched
    return m.group(i) if m.group(i) is not None else m.group(i + 1)

def block(text, name):
    m = re.search(r"const " + name + r"\s*=\s*<[^>]*>\[(.*?)\n\];", text, re.S)
    if not m:
        sys.exit(f"missing {name}")
    return m.group(1)

def towns(text, name):
    out = []
    for m in TOWN_RE.finditer(block(text, name)):
        g = m.groups()
        # groups: id(2) name(2) division(2) pop wealth turnout loyalty culture biz cap nick(2) mascot(2) quirk(2)
        out.append(dict(
            id=g[0] or g[1], name=g[2] or g[3], division=g[4] or g[5], population=int(g[6]), wealth=int(g[7]),
            turnout=int(g[8]), loyalty=int(g[9]), culture=int(g[10]), biz=int(g[11]), capacity=int(g[12]),
            nickname=g[13] or g[14] or "", mascot=g[15] or g[16], quirk=g[17] or g[18]))
    return out

def rivalries(text, name):
    return [list(p) for p in re.findall(r"\('(\w+)',\s*'(\w+)'\)", block(text, name))]

regions = {}
meta = {
    "ca": dict(name="California", divisions=["North", "Coastal", "South"], coastal="Coastal", cold=False),
    "tx": dict(name="Texas", divisions=["Plains", "Gulf", "Central"], coastal="Gulf", cold=False),
    "fl": dict(name="Florida", divisions=["North", "Gulf", "Atlantic"], coastal="Gulf", cold=False),
    "pa": dict(name="Pennsylvania", divisions=["West", "Central", "East"], coastal="", cold=True),
}
for rid in meta:
    text = (CORE / f"{rid}_region.dart").read_text()
    regions[rid] = dict(id=rid, **meta[rid], starting=towns(text, f"{rid}StartingTowns"),
                        replacement=towns(text, f"{rid}ReplacementTowns"), rivalries=rivalries(text, f"{rid}Rivalries"))
    r = regions[rid]
    assert len(r["starting"]) == 12 and len(r["replacement"]) == 15, (rid, len(r["starting"]), len(r["replacement"]))

ca = (CORE / "ca_region.dart").read_text()
nick = re.findall(STR, re.search(r"const nicknameTable = <String>\[(.*?)\];", ca, re.S).group(1))
nicknames = [a or b for a, b in nick]
nr = re.search(r"const nicknameRegions = <String, List<String>>\{(.*?)\};", ca, re.S).group(1)
nickname_regions = {(a or b): re.findall(r"'(\w+)'", rest)
                    for a, b, rest in re.findall(STR + r":\s*\[(.*?)\]", nr)}
mascots = [a or b for a, b in re.findall(STR, re.search(r"const mascotTable = <String>\[(.*?)\];", ca, re.S).group(1))]

town_dart = (CORE / "town.dart").read_text()
abbr = dict(re.findall(r"'(\w+)':\s*'(\w+)'", re.search(r"const townAbbreviations = <String, String>\{(.*?)\n\};", town_dart, re.S).group(1)))

geo = (CORE / "geo.dart").read_text()
coords = {k: [float(a), float(b)] for k, a, b in
          re.findall(r"'([a-z. \-]+)':\s*\(\s*(-?\d+\.\d+),\s*(-?\d+\.\d+)\s*\)", re.search(r"_coords = <String, \(double, double\)>\{(.*?)\n\};", geo, re.S).group(1))}

(OUT / "regions.json").write_text(json.dumps(regions, indent=1))
(OUT / "nicknames.json").write_text(json.dumps(dict(nicknames=nicknames, regions=nickname_regions, mascots=mascots), indent=1))
(OUT / "town_meta.json").write_text(json.dumps(dict(abbreviations=abbr, coords=coords), indent=1))
for src, dst in [("app/assets/jobs/semi_pro_football_jobs.csv", "jobs.csv"), ("app/assets/names/football_male_first_names.csv", "first_names.csv"),
                 ("app/assets/names/football_male_last_names.csv", "last_names.csv"), ("app/assets/sizes/football_position_sizes.csv", "position_sizes.csv")]:
    shutil.copy(SRC / src, OUT / dst)

# ---- economy tables (sp_economy/business.dart) ----
ECON = SRC / "packages/sp_economy/lib/src/business.dart"
biz = ECON.read_text()
RSTR = r"""(?:r?'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")"""

def unq(m, i):
    return m[i] if m[i] is not None else m[i + 1]

def named(args):
    return {k: float(v) for k, v in re.findall(r"(\w+):\s*(-?[\d.]+)", args)}

clauses = []
cb = re.search(r"const sponsorClauses = <SponsorClause>\[(.*?)\n\];", biz, re.S).group(1) + "\n"
for m in re.finditer(r"SponsorClause\(\s*" + RSTR + r",\s*" + RSTR + r",\s*([\d.]+)(.*?)\),?\n", cb, re.S):
    g = m.groups()
    n = named(g[5])
    clauses.append(dict(id=g[0] or g[1], text=g[2] or g[3], payMult=float(g[4]), minWinPct=n.get("minWinPct", 0.0), fanPerGame=n.get("fanPerGame", 0.0),
                        attMult=n.get("attMult", 1.0), costPerGame=int(n.get("costPerGame", 0)), winBonus=int(n.get("winBonus", 0))))
assert len(clauses) == 27, len(clauses)

promos = []
pb = re.search(r"const promoTemplates = <PromoTemplate>\[(.*?)\n\];", biz, re.S).group(1) + "\n"
for m in re.finditer(r"PromoTemplate\(" + RSTR + r",\s*" + RSTR + r",\s*(\d+),\s*([\d.]+),\s*([\d.]+),\s*([\d.]+),\s*([\d.]+),\s*" + RSTR + r"(.*?)\),?\n", pb, re.S):
    g = m.groups()
    n = named(g[11])
    promos.append(dict(id=g[0] or g[1], name=g[2] or g[3], baseCost=int(g[4]), attendanceMult=float(g[5]), perCapMult=float(g[6]), merchMult=float(g[7]),
                       riskChance=float(g[8]), riskText=g[9] or g[10], mishapAttMult=n.get("mishapAttMult", 0.88), mishapFineMult=n.get("mishapFineMult", 0.0)))
assert len(promos) == 6, len(promos)

facs = []
fb = re.search(r"const facilitySpecs = <Facility, FacilitySpec>\{(.*?)\n\};", biz, re.S).group(1)
for m in re.finditer(r"Facility\.(\w+): FacilitySpec\(" + RSTR + r",\s*(\d+),\s*(\d+),\s*(\d+),\s*" + RSTR + r"\)", fb):
    g = m.groups()
    facs.append(dict(key=g[0], name=g[1] or g[2], baseCost=int(g[3]), maxLevel=int(g[4]), upkeepPerLevel=int(g[5]), effect=g[6] or g[7]))
assert len(facs) == 5, len(facs)

names = [a or b for a, b in re.findall(RSTR, re.search(r"static const _names = \[(.*?)\];", biz, re.S).group(1))]
assert len(names) == 50, len(names)
(OUT / "economy.json").write_text(json.dumps(dict(sponsorClauses=clauses, promos=promos, facilities=facs, sponsorNames=names), indent=1))
print(f"economy: clauses={len(clauses)} promos={len(promos)} facilities={len(facs)} sponsorNames={len(names)}")

print(f"regions={list(regions)} nicknames={len(nicknames)} regionTags={len(nickname_regions)} mascots={len(mascots)} abbr={len(abbr)} coords={len(coords)}")
