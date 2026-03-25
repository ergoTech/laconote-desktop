# Backend Tasks for Laconote Desktop Integrations

## Task 1: Transcript Export API

**Priority:** High
**Endpoint:** `GET /api/v1/meetings/{meeting_id}/transcript`

**Description:**
Десктоп додаток та інтеграції (Notion, Google Docs, Slack) потребують доступ до повного транскрипту зустрічі. Наразі транскрипт доступний тільки через веб-дашборд.

**Request:**
```
GET /api/v1/meetings/{meeting_id}/transcript
Authorization: Bearer {jwt}
```

**Response:**
```json
{
  "meeting_id": "uuid",
  "meeting_name": "Q4 Planning",
  "meeting_type": "technical",
  "duration_seconds": 3600,
  "transcript": {
    "full_text": "...",
    "segments": [
      {
        "speaker": "Speaker 1",
        "start": 0.0,
        "end": 15.5,
        "text": "Let's start with the agenda..."
      }
    ]
  },
  "summary": {
    "text": "...",
    "action_items": ["..."],
    "key_points": ["..."]
  }
}
```

**Використання:**
- Десктоп: експорт в Notion/Google Docs
- Slack webhook: надсилання summary після обробки
- Zapier: payload для автоматизацій

---

## Task 2: Webhook After Transcription

**Priority:** High
**Trigger:** Після завершення обробки кожної зустрічі (всі чанки оброблені Whisper + summary згенерований)

**Description:**
Після завершення транскрипції бекенд надсилає POST webhook на URL налаштований юзером. Це дозволяє інтеграції зі Slack, Zapier, n8n, та будь-якими custom автоматизаціями.

**User Config (нове поле в user settings):**
```json
{
  "webhook_url": "https://hooks.slack.com/services/xxx/yyy/zzz",
  "webhook_enabled": true,
  "webhook_events": ["meeting.completed"]
}
```

**Webhook Payload:**
```json
{
  "event": "meeting.completed",
  "meeting_id": "uuid",
  "meeting_name": "Q4 Planning",
  "meeting_type": "technical",
  "meeting_url": "https://laconote.com/meeting/uuid",
  "duration_seconds": 3600,
  "started_at": "2026-03-25T12:00:00Z",
  "completed_at": "2026-03-25T13:05:00Z",
  "summary": "Team discussed Q4 priorities...",
  "action_items": [
    "Review budget proposal by Friday",
    "Schedule follow-up with design team"
  ],
  "key_points": [
    "Q4 budget approved",
    "New hire starts next month"
  ],
  "speakers": ["Speaker 1", "Speaker 2"],
  "transcript_url": "https://meet.laconote.com/api/v1/meetings/uuid/transcript"
}
```

**Implementation:**
1. Додати `webhook_url` та `webhook_enabled` до user settings model
2. Після завершення транскрипції (останній чанк оброблений) → сформувати payload
3. POST на `webhook_url` з retry (3 спроби, exponential backoff)
4. Логувати результат (success/failure) в meeting metadata
5. UI в дашборді: Settings → Integrations → Webhook URL + test button

---

## Task 3: Slack Integration (Server-Side)

**Priority:** High
**Dependencies:** Task 2 (Webhook)

**Description:**
Нативна Slack інтеграція: після обробки зустрічі надсилати summary в Slack канал. Потребує Slack OAuth на бекенді.

**Flow:**
1. User натискає "Connect Slack" в дашборді
2. OAuth2 flow: `https://slack.com/oauth/v2/authorize` з scope `chat:write`, `channels:read`
3. Бекенд зберігає `slack_access_token` та `slack_channel_id` в user settings
4. Після транскрипції → `POST https://slack.com/api/chat.postMessage` з formatted summary

**Slack Message Format:**
```
📝 *Meeting Summary: Q4 Planning*
Duration: 1h 00m | 2 speakers

*Key Points:*
• Q4 budget approved
• New hire starts next month

*Action Items:*
• Review budget proposal by Friday
• Schedule follow-up with design team

<https://laconote.com/meeting/uuid|View full transcript →>
```

**Endpoints:**
- `GET /api/v1/integrations/slack/connect` → redirect to Slack OAuth
- `GET /api/v1/integrations/slack/callback` → handle OAuth callback
- `DELETE /api/v1/integrations/slack/disconnect`
- `GET /api/v1/integrations/slack/channels` → list user's Slack channels
- `PUT /api/v1/integrations/slack/config` → set target channel

---

## Task 4: Improve Whisper Accuracy

**Priority:** Medium

**Description:**
Транскрипція показує сміття на початку запису та погану якість для українських/змішаних розмов.

**Можливі покращення:**
1. **Language hint:** Передавати `language` параметр в Whisper. Десктоп може передавати detected language в chunk metadata
2. **Initial prompt:** Передавати контекст зустрічі (meeting name, type) як initial prompt для Whisper
3. **VAD preprocessing:** Вирізати тишу перед відправкою в Whisper
4. **Chunk overlap:** Додати 1-2с overlap між чанками щоб уникнути обрізання слів на границях
5. **Model upgrade:** Використовувати whisper-large-v3 замість поточної моделі

**Нове поле в chunk upload:**
```json
{
  "language_hint": "uk",
  "meeting_context": "Technical standup about Q4 planning"
}
```

---

## Task 5: Speaker Diarization

**Priority:** Medium

**Description:**
Наразі всі слова приписуються "Speaker 1". Потрібна діаризація (хто говорить) для якісних транскрипцій.

**Approaches:**
1. **pyannote-audio** — найпопулярніша бібліотека для діаризації
2. **NeMo** — NVIDIA модель, висока якість
3. **Post-processing:** Після Whisper, використати embedding-based clustering

**Інтеграція:**
- Після Whisper транскрипції, запустити діаризацію на тому ж аудіо
- Мерджити word-level timestamps з speaker segments
- Зберегти в `transcript.segments[].speaker` як "Speaker 1", "Speaker 2" etc

---

## Task 6: Recording Status Sync

**Priority:** Low

**Description:**
Десктоп додаток відправляє "recording started/stopped" events. Бекенд може показувати real-time status в дашборді.

**Нові endpoints:**
- `POST /api/v1/meetings/{id}/status` — `{ "status": "recording" | "stopped" }`
- `GET /api/v1/meetings/{id}/status` — для дашборду

**WebSocket option:**
- `wss://meet.laconote.com/ws/meeting/{id}` — real-time status updates

---

## Task 7: Meeting Search API

**Priority:** Low

**Description:**
Full-text search по всіх транскрипціях юзера. Дозволяє знайти "коли ми обговорювали бюджет" серед всіх зустрічей.

**Endpoint:**
```
GET /api/v1/meetings/search?q=бюджет&limit=20
```

**Response:**
```json
{
  "results": [
    {
      "meeting_id": "uuid",
      "meeting_name": "Q4 Planning",
      "date": "2026-03-25",
      "snippet": "...обговорили бюджет на наступний квартал...",
      "timestamp_seconds": 1234.5
    }
  ]
}
```

**Implementation:** PostgreSQL full-text search або Elasticsearch/Meilisearch.

---

## Summary Table

| # | Task | Priority | Dependencies | Effort |
|---|------|----------|-------------|--------|
| 1 | Transcript Export API | High | None | Small |
| 2 | Webhook After Transcription | High | None | Medium |
| 3 | Slack Integration | High | Task 2 | Medium |
| 4 | Improve Whisper Accuracy | Medium | None | Medium |
| 5 | Speaker Diarization | Medium | Task 4 | Large |
| 6 | Recording Status Sync | Low | None | Small |
| 7 | Meeting Search API | Low | None | Medium |
