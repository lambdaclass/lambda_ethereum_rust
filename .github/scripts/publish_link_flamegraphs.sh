curl -XPOST -H "Content-type: application/json" -d '{
  "blocks": [
    {
      "type": "header",
      "text": {
        "type": "plain_text",
        "text": "Daily Flamegraph Report"
      }
    },
    {
      "type": "divider"
    },
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "🔥 Flamegraphs are available at *<https://lambdaclass.github.io/ethrex/|lambdaclass.github.io/ethrex/>*"
      }
    }
  ]
}' "$url"
