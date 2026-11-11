# Meta sources: what the pipeline may fetch, and why

Plan section 15.2. Every source the meta pipeline touches is listed here with the terms that allow it. A source whose terms forbid automated collection or republishing is dropped, never worked around. Reviewed 2026-11-12.

## Approved

| Source | Used for | Access | Terms |
|---|---|---|---|
| **valorant-api.com** | Map minimaps, callouts, map URLs, agent list, game version (`meta-pipeline/sync-assets.mjs`) | Public JSON API, one request per endpoint per weekly run | Community API meant for this. Assets remain Riot's property; we store coordinates and names, not images. |
| **LiquipediaDB API** | Tier 1: pro agent pick and win rates per map, observed 5-agent comps (`meta-pipeline/liquipedia.mjs`) | `api.liquipedia.net/api/v3/match`, API key in the `LIQUIPEDIA_API_KEY` repository secret, at most 6 paged requests per weekly run, 2.5 s apart, identifying User-Agent | [API terms of use](https://liquipedia.net/api-terms-of-use): content is **CC-BY-SA 3.0**, attribution required (shipped in `data/meta-tier1.json` and shown in the app); API access "upon approved request"; cache results, stay inside the plan's rate limit, don't share the key. Automated access to Liquipedia's HTML pages is **not** permitted, so we never touch them. |

Liquipedia data is CC-BY-SA 3.0, so `data/meta-tier1.json` (the derived aggregate) is shared under the same licence, separately from the app's MIT/Apache code licence.

## Excluded

| Source | Why |
|---|---|
| **VLR.gg** | Terms of Service ("Prohibited activities") forbid systematically retrieving data to build a collection or database and using any scraper or robot. No API. Excluded, including through third-party proxies of its data. |
| **Metabot.gg** | Terms allow personal, non-commercial use and state the site's data compilations are its own property. Republishing its numbers in our dataset isn't covered. Excluded unless they grant permission. |
| **Tracker.gg Insights** | Returns 403 to automated requests. Not bypassed. Their developer API could be asked whether it includes insights data. |
| **vstats.gg** | Renders only with JavaScript and its robots.txt disallows the filtered views we'd need. Manual reference only. |
| **ValoHub, tier-list articles** | Estimates or editorial picks, not measured data. Sanity checks during human review only. |
| **HenrikDev** | Bulk-collecting other players' matches for statistics breaks its rules. The pipeline never calls it. |

## Tier 2 (ranked, all maps)

No source found offers ranked agent or comp statistics that we may fetch and republish. Tier 2 stays empty; the advisor says when its suggestions come from role coverage only. If a source with suitable terms or an agreement appears, it gets an adapter and a row in the table above first.

## Enabling Tier 1

1. Request a LiquipediaDB API key (Liquipedia's API page, or their Discord). Describe the use: a weekly, cached aggregate for an open-source app, with attribution.
2. Add it as the `LIQUIPEDIA_API_KEY` repository secret.
3. Run the **Meta pipeline** workflow (Actions → Meta pipeline → Run workflow). It opens a pull request with the new `data/meta-tier1.json` and a summary of the changes; merging it publishes a signed dataset the app picks up within a day.

If the API's response shape changes, the adapter fails the run with a sample of what it saw instead of publishing empty data.
