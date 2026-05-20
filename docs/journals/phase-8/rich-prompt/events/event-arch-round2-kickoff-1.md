{
  "id": "arch-round2-kickoff-1",
  "type": "survey",
  "from": "@@Architect",
  "to": "@@Alex",
  "topic": "Round-2 kickoff — which agents to spawn + when to cut task files",
  "questions": [
    {
      "header": "Spawn",
      "text": "Which agents to spawn now for Round-2 kickoff?",
      "options": [
        { "key": "A", "label": "All six (parallel ramp; FullStackA + WebtestA/B get Wave-2 standby tasks)" },
        { "key": "B", "label": "Wave-1 north-star three only (@@CI + @@Systacean + @@FullStackB)" },
        { "key": "C", "label": "Custom (you say in chat which agents + tasks)" }
      ]
    },
    {
      "header": "Tasks",
      "text": "When should I cut the Wave-1 task files?",
      "options": [
        { "key": "1", "label": "Cut now — agents see tasks on first bootstrap read" },
        { "key": "2", "label": "At fan-out — after agents bootstrap + ack their identity" }
      ]
    }
  ]
}
