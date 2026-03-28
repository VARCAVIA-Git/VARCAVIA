# Spider Audit Report

**Date:** 2026-03-28 00:30 UTC
**Uptime at measurement:** ~3 minutes after deploy

## Facts Overview

| Metric | Value |
|--------|-------|
| Total facts in DB | 746 |
| Hardcoded seed facts | 482 |
| Spider-discovered facts | 264 |
| Topics crawled | 52 |
| Attestations added | 6 |
| Contradictions flagged | 324 |
| Growth rate | ~160 new facts/minute |

## Tier Distribution

| Tier | Count |
|------|-------|
| T0 | 0 |
| T1 | 544 |
| T2 | 1 |
| T3 | 0 |
| T4 | 0 |

## Sources

| Source | Status | Notes |
|--------|--------|-------|
| Wikipedia REST API | Working | Extracts 5-10 factual sentences per topic |
| Wikidata SPARQL | Working but noisy | Returns technical metadata as facts |
| REST Countries | Not implemented | Was in the spec but not in spider.rs |

## Quality Assessment

### Good Facts (from Wikipedia)
- "Electricity is the set of physical phenomena associated with the presence and motion..." - Well-formed, factual
- "A computer is a machine that can be programmed to automatically carry out sequences..." - Good
- "The number of deaths of World War II is 73000000." - Factual (from Wikidata)

### Bad Facts (from Wikidata)
- "The height of Computer is 21." - Infobox image height in pixels
- "The width of Computer is 16." - Infobox image width in pixels
- "The duration of Electricity is 210." - Meaningless
- "The duration of Internet is 101." - Meaningless
- "The inception of Internet is 1969-10-29T00:00:00Z." - Raw ISO date, not human-readable

### Root Causes

1. **Wikidata noise**: The SPARQL query returns ALL properties including technical metadata (image dimensions, template IDs, Commons categories). These need filtering.

2. **Contradiction over-counting**: 324 contradictions in 3 minutes is excessive. The `detect_contradiction` function flags any two facts that share keywords AND have different numbers — this catches unrelated facts like "Earth radius is 6371" and "Earth population is 8 billion" because they share "Earth" and have different numbers.

3. **No Wikidata property filtering**: Properties like "height", "width", "duration", "number of pages", "Commons category" are technical, not factual. Need a blocklist.

4. **Date formatting**: Wikidata dates come as ISO timestamps (1969-10-29T00:00:00Z) but get stored as-is instead of being formatted as "October 29, 1969".

## Diagnosis

| Question | Answer |
|----------|--------|
| Topics crawled | 52 in ~3 min (1 per second + API latency) |
| Seeds per topic | ~10 from Wikipedia concept extraction |
| Queue growing? | Yes — expanding exponentially from initial 28 |
| API errors? | Some Wikidata 403s (handled gracefully) |
| Rate limiting? | Yes, 1 sec between topics working |
| Expanding or circling? | Expanding — visited set prevents revisits |

## Speed

- **Current rate:** ~5 new facts/second (~300/minute)
- **Topics/hour:** ~1,800 (limited by 1 sec/topic + 1 sec Wikidata delay)
- **Facts/hour:** ~9,000 (at ~5 facts/topic)
- **Projection 48h (Tuesday):** ~430,000 new facts (if queue doesn't empty)

## Critical Fixes Needed

### P0: Wikidata property blocklist
Filter out technical properties: height, width, duration, number of pages, aspect ratio, Commons category, image, Wikimedia, template, taxon.

### P1: Contradiction logic fix
Require SAME UNIT/ATTRIBUTE for contradiction, not just same subject. "Earth radius" vs "Earth population" is NOT a contradiction.

### P2: Date formatting
Convert "1969-10-29T00:00:00Z" to "October 29, 1969" or at least "1969".

### P3: Fact minimum quality
Skip Wikidata facts where the value is just a bare number without context (like "21", "210", "101").
