{
  "id": "arch-survey-1",
  "type": "survey",
  "from": "@@Architect",
  "to": "@@Alex",
  "topic": "broadcast smoke test — survey path",
  "questions": [
    {
      "header": "Bubble",
      "text": "Does the survey bubble render cleanly in the rich prompt?",
      "options": [
        { "key": "Y", "label": "Yes, all good" },
        { "key": "N", "label": "No, something looks off" }
      ]
    },
    {
      "header": "Numbered",
      "text": "Does pressing the option key (Y / N) reply without leaking the keystroke into the prompt buffer?",
      "options": [
        { "key": "1", "label": "Yes, clean" },
        { "key": "2", "label": "Leaks into prompt" }
      ]
    }
  ]
}
