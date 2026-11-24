---
name: using-asep
description: Use when the agent starts a session and must load the ASEP ambient skills listed in the pointer block, then apply capability routing to route each task to the correct skill.
---

# using-asep

Load the ASEP ambient skills named in the session pointer block at startup. Each listed skill becomes available for capability routing, which routes tasks to the skill whose scope matches the task. Do not invent skills outside the pointer block. Follow Tier T capability routing; unmatched intents exit 3. When routing, match the task intent to a skill id, then execute that skill's procedure. If no skill matches, surface the gap instead of forcing a fit. Keep routing decisions traceable by recording the pointer block entry and the chosen skill id for each task.
