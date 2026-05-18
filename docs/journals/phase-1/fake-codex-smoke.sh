#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"thread.started","thread_id":"thread_webtest"}'
sleep 2
printf '%s\n' '{"type":"item.completed","item":{"id":"msg_webtest","type":"agent_message","text":"assistant smoke ok with a deliberately long response body that should stretch across the available chat column instead of staying as a narrow bubble."}}'
printf '%s\n' '{"type":"turn.completed","turn":{"status":"completed"}}'
