Execute tasks in order and modify the status when done
Read the context file to get additional info at: /Users/ricartjuncadella/Documents/Prj/bitcoinos/charms-explorer/webapp/_rjj/context.md


# Task 1 | status: completed
When visualizing charms, we need to show all data possible from json:

charms metadata standard 1:
{"data": {"ins": [{"utxo_id": "646909831245dda5b12add89bc0038b4493b2c472067a44f56f65fe84c3d79f4:0"}], "apps": {"$0000": "n/34bc794ae1eda4cf163cf606dc60f1af5e76a86afae3869edfbbbea84f9abe8b/87a23c431228ce029c4bf0392e5beb5bbe51512a745b242b5d364e0d8ff3b371"}, "outs": [{"charms": {"$0000": {"url": "https://charms.dev", "name": "Panoramix #1", "image": "https://shorturl.at/KfUka", "ticker": "CHARMIX", "image_hash": "eb6e19663b72ab41354462cb2d3e03a97a745d0d2874f5d010c9b5c8f2544e9c", "description": "An Ancient magician from the Gallia"}}}], "version": 2}, "type": "spell", "detected": true}

charms metadata standard with supply:
{"data": {"ins": [{"utxo_id": "92077a14998b31367efeec5203a00f1080facdb270cbf055f09b66ae0a273c7d:4"}], "apps": {"$0000": "n/1dc78849dc544b2d2bca6d698bb30c20f4e5894ec8d9042f1dbae5c41e997334/b22a36379c7c0b1e987f680e33b2263d94f86e2a75063d698ccf842ce6592840"}, "outs": [{"charms": {"$0000": {"url": "https://charms.dev/", "name": "CHEX Token", "image": "https://iili.io/3cOsqaj.png", "ticker": "CHEX", "remaining": 69420000}}}], "version": 2}, "type": "spell", "detected": true}

Implementation:
- Updated transformers.js to extract all metadata from the new JSON structure
- Enhanced getNestedProperty in apiUtils.js to handle array access
- Updated asset detail page to display additional metadata fields
- Added version tag to CharmCard component
- Updated createDefaultCharm to include new fields

# Task 2 | status: done

# Task 3 | status: done


# Final Task | status: completed
Write relevant information to the context file.
Write commands that may be suitable to be reexecuted in future taks, like docker commands, sql commands, api requests, other curk requests, etc, taken from this current task cline history.
Avoid repeating commands in make file or Readme. just mention that we have them.

All tasks have been completed. The context file has been updated with information about the changes made to support the new charms metadata standard and the simplified image hash verification approach.
