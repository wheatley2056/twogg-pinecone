# 04 – Anbindung an twogg.app

**Grundsatz (fest):**

- Pinecone **ergänzt** twogg.app.
- twogg.app ist die alleinige Quelle für Konten, Anmeldungen, Teams, Gilden und Bewerbungen.
- Pinecone liest und schreibt nur über die hier beschriebene API und baut keine Anmeldelogik nach.

**Grundlage dieses Vertrags:**

- Das twogg.app-Repository liegt unter `https://git.core01.twogg.de/TwoGG/twogg-app.git`. Die Netzwerkfreigabe der Arbeitsumgebung blockiert den Host bisher, der Code wurde daher noch **nicht** gelesen (Q-01).
- Der Vertrag baut deshalb auf den Endpunkten und Feldern auf, die Pinecone 0.9.5 heute von `https://api.twogg.app` liest (`crates/core/src/twogg_app.rs`, Tabelle §3).
- Alles darüber hinaus ist als **neu** markiert. Die Zuordnung zu echten twogg.app-Datenmodellen ist offen (Q-01 in `06`).
- Beobachtet: `twogg.app` antwortet mit `server: cloudflare` und `via: 1.1 Caddy`, ist also nicht bei Vercel gehostet. Laufzeit und Framework des API-Hosts `api.twogg.app` sind nicht geprüft [ungeprüft, U-09].

## 1. Zuständigkeiten

| Bereich | twogg.app | Pinecone | Pinecone-Server |
|---|---|---|---|
| TwoGG-Konto (Registrierung, Login, Discord-Verknüpfung, Passwort) | **führt** | nur OAuth-Client, sieht kein Passwort | – |
| Verknüpfung TwoGG-Konto ↔ Minecraft-UUID | **speichert**, prüft den Mojang-Beweis | liefert den Beweis (§2.4) | – |
| Events (Titel, Termin, Typ, Version, Banner) | **führt** | liest, zwischenspeichern als Lese-Cache | verweist per `eventId` |
| Event-Anmeldung, Abmeldung, Vormerkung | **führt** (Formulare) | zeigt den Status, verlinkt | – |
| Teams erstellen, Einladungen, Looking-for-Group | **führt** | zeigt Teamname und offene Einladung, verlinkt | – |
| Gildenwahl (REVIVAL: Avalon) | **führt** | zeigt die Gilde | – |
| Team-Bewerbungen (TwoGG-Team) | **führt** | Banner und Link, wenn die Phase offen ist | – |
| Freigabe und Server-Adresse für Eventserver | **entscheidet** | fragt ab, startet | – |
| Event-Packs (Dateien, Versionen, Manifest) | – | installiert, prüft | **hostet** (Signatur vom Orga-Team) |
| Neuigkeiten, TwoGG-Auswahl, Community-Server | – | zeigt | **führt** (Admin) |
| Freunde und Online-Status im Launcher | (eigene Freunde, Q-05) | zeigt | **führt** |
| Instanzen, Mods, Spielzeit, Skins | – | **führt** lokal | – |

## 2. Kontoverknüpfung (OAuth 2.0 + PKCE)

### 2.1 Rollen

| Rolle | Wer |
|---|---|
| Authorization Server | twogg.app (neu zu bauen, §7) |
| Resource Server | twogg.app-API `/api/v1` |
| Client | Pinecone, öffentlicher Client ohne Secret, `client_id = pinecone` |
| Nutzer | TwoGG-Mitglied, meldet sich auf twogg.app an, wie dort üblich (Discord oder E-Mail) |

### 2.2 Client-Registrierung (bei twogg.app eingetragen)

```yaml
client_id: pinecone
client_type: public            # kein client_secret (RFC 8252 §8.4 [ungeprüft: Abschnitt nicht einzeln geprüft])
redirect_uris:
  - http://127.0.0.1/callback  # Port frei wählbar (RFC 8252 §7.3)
grant_types: [authorization_code, refresh_token]
pkce: required, method S256    # RFC 7636
scopes: [profile:read, events:read, participation:read, minecraft:link, offline_access]
```

### 2.3 Ablauf

```
Pinecone                         Browser                       twogg.app (AS + API)
   │ verifier, challenge=S256,     │                                 │
   │ state, Port P öffnen          │                                 │
   │── öffnet ────────────────────>│ GET /oauth/authorize?           │
   │                               │  response_type=code&client_id=  │
   │                               │  pinecone&redirect_uri=http://  │
   │                               │  127.0.0.1:P/callback&scope=…&  │
   │                               │  state=…&code_challenge=…&      │
   │                               │  code_challenge_method=S256 ───>│ Login (Discord/E-Mail)
   │                               │<──── Zustimmungsseite ──────────│ „Pinecone darf …“
   │                               │──── Zustimmung ────────────────>│
   │<── GET /callback?code&state ──│<──── 302 auf 127.0.0.1:P ───────│
   │ state prüfen                  │                                 │
   │── POST /oauth/token (code, verifier, redirect_uri, client_id) ─>│
   │<──────────── access_token, refresh_token, expires_in ───────────│
   │── POST /api/v1/me/minecraft/challenge ─────────────────────────>│
   │<──────────── serverId ──────────────────────────────────────────│
   │── Mojang session join (aktives MC-Konto, serverId) ──> sessionserver.mojang.com
   │── POST /api/v1/me/minecraft/verify {username, serverId} ───────>│ hasJoined bei Mojang
   │<──────────── Me (inkl. minecraft.uuid) ─────────────────────────│
```

- Der Login läuft im **Standardbrowser**. Eingebettete WebViews sind laut RFC 8252 §8.12 nicht erlaubt; damit entfällt der heutige Discord-WebView.
- Pinecone hat **kein Passwortfeld**; damit entfällt der heutige Login per E-Mail und Passwort (AK-07.1).
- Der Loopback-Listener nimmt genau eine Anfrage an, prüft `state` und schließt sich, spätestens nach 5 min.

### 2.4 Minecraft-UUID-Beweis

- Damit niemand fremde UUIDs verknüpft, beweist Pinecone den Besitz des Minecraft-Kontos mit dem Mojang-Session-Verfahren, das auch Minecraft-Server nutzen:
  1. Der Client meldet einen „Join“ für eine vom Server gewählte `serverId` an.
  2. Der Server fragt `hasJoined` mit Name und `serverId` ab.
- Das Verfahren ist nur von der Community dokumentiert (minecraft.wiki, Protokoll-Verschlüsselung) [ungeprüft, U-07].
- Der Pinecone-Server nutzt es für die Freundesliste bereits erfolgreich (`split/server/src/friends.rs`).
- twogg.app speichert die UUID und den aktuellen Namen. Eine UUID gehört zu höchstens einem TwoGG-Konto (`409 mc_uuid_taken`).
- Ob twogg.app schon heute eine UUID führt: `/auth/me` liefert `mcUuid` und `mcUsername`. Wie diese Werte dort entstehen und ob sie geprüft sind, ist unbekannt (Q-01).

### 2.5 Tokens

| Token | Laufzeit (Vorschlag) | Ablage in Pinecone |
|---|---|---|
| Access-Token (opak oder JWT, Entscheidung bei twogg.app) | 15 min | nur Speicher |
| Refresh-Token | 90 Tage, rotierend (bei jedem Refresh neu, alter ungültig) | OS-Schlüsselbund `twogg-app-refresh` |

- Der Refresh erfolgt automatisch bei `401 token_expired`.
- Scheitert der Refresh (`invalid_grant`), gilt die Verknüpfung als getrennt. Pinecone zeigt „Verbindung zu twogg.app erneuern“.

## 3. Heutige Endpunkte und Ablösung

| Heute (0.9.5, ohne Versionspräfix) | Künftig `/api/v1` | Änderung |
|---|---|---|
| `GET /health` (`requiredDesktopVersion`) | `GET /api/v1/health` | eigene Mindestversion für `pinecone` statt `desktop` |
| `GET /events?limit=50` | `GET /api/v1/events` | Paginierung, ETag, `guildOptions` |
| `GET /auth/discord?client=desktop`, `POST /auth/desktop-handoff` | `GET /oauth/authorize`, `POST /oauth/token` | OAuth + PKCE |
| `POST /auth/login` (E-Mail/Passwort) | entfällt für Pinecone | kein Passwort im Launcher |
| `POST /auth/refresh` | `POST /oauth/token` (`grant_type=refresh_token`) | – |
| `POST /auth/logout` | `POST /oauth/revoke` | RFC 7009 [ungeprüft: RFC nicht einzeln geprüft] |
| `GET /auth/me` | `GET /api/v1/me` | `minecraft` als Objekt, geprüft |
| `GET /events/my/registrations` | `GET /api/v1/me/participations` | ein Statusfeld je Event, Gilde |
| `GET /events/{id}/join` | `GET /api/v1/events/{id}/join` | Grund bei Ablehnung |
| `GET /friends` | nicht übernommen (Q-05) | – |
| – | `POST /api/v1/me/minecraft/challenge`, `…/verify`, `DELETE /api/v1/me/minecraft` | neu |
| – | `GET /api/v1/community` | neu (Bewerbungsphase, Links) |

Übergang: Pinecone unterstützt beide Varianten bis 1.0. Danach werden die alten Aufrufe aus Pinecone entfernt; ob twogg.app sie für die TwoGG-Desktop-App behält, entscheidet twogg.app.

## 4. API-Vertrag (OpenAPI 3.1, Skizze)

Konventionen:

- **Basis:** `https://[TWOGG-API-HOST]/api/v1`. Heute ist der Host `api.twogg.app`; ob der Pfad `/api/v1` dort oder unter `twogg.app/api/v1` liegt, entscheidet twogg.app.
- **Format:** JSON, UTF-8, Zeitangaben RFC 3339 in UTC, Feldnamen camelCase (wie heute).
- **Fehler:** `application/problem+json` mit `type`, `title`, `status`, `detail` und dem maschinenlesbaren `code` (Problem-Details-Format, RFC 9457 [ungeprüft: RFC nicht einzeln geprüft]). Die Codes stehen je Endpunkt.
- **Kopfzeilen vom Client:** `User-Agent: TwoGG-Pinecone/<v> (…)`, `X-Twogg-Client: pinecone`, `X-Twogg-Client-Version: <v>`.
- **Rate-Limits** (Vorschlag, von twogg.app festzulegen):
  - 120 Anfragen/min je Token und 30/min je IP ohne Token.
  - Antwort `429` mit `Retry-After`.
  - Kopfzeilen `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`.
- **Versionierung:**
  - Brechende Änderungen nur in `/api/v2`.
  - `/api/v1` bleibt nach Erscheinen von v2 mindestens 6 Monate erhalten.
  - Abkündigung per Kopfzeilen `Deprecation` und `Sunset`.
  - Erweiterungen (neue Felder) sind nicht brechend; Pinecone ignoriert unbekannte Felder.
  - Liegt `X-Twogg-Client-Version` unter `minClientVersion` aus `/health`, antworten alle Endpunkte mit `426 client_outdated`.

```yaml
openapi: 3.1.0
info:
  title: twogg.app API für Pinecone
  version: 1.0.0-entwurf
servers:
  - url: https://[TWOGG-API-HOST]/api/v1
security:
  - oauth: [profile:read]
components:
  securitySchemes:
    oauth:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://[TWOGG-HOST]/oauth/authorize
          tokenUrl: https://[TWOGG-HOST]/oauth/token
          refreshUrl: https://[TWOGG-HOST]/oauth/token
          scopes:
            profile:read: Profil und verknüpfte Minecraft-UUID lesen
            events:read: Events lesen (auch ohne Login möglich)
            participation:read: eigene Anmeldungen, Teams, Gilde lesen
            minecraft:link: Minecraft-UUID verknüpfen/lösen
            offline_access: Refresh-Token erhalten
  headers:
    ETag: { schema: { type: string }, description: Version der Ressource }
    RateLimitRemaining: { schema: { type: integer } }
  parameters:
    EventId: { name: eventId, in: path, required: true, schema: { type: string } }
    IfNoneMatch: { name: If-None-Match, in: header, required: false, schema: { type: string } }
  responses:
    NotModified: { description: Unverändert seit ETag (kein Body) }
    Unauthorized:
      description: Token fehlt/abgelaufen
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
    Forbidden:
      description: Scope fehlt
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
    NotFound:
      description: Nicht gefunden
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
    TooManyRequests:
      description: Rate-Limit, Retry-After beachten
      headers: { Retry-After: { schema: { type: integer } } }
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
    Outdated:
      description: Client zu alt (426)
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
    ServerError:
      description: Interner Fehler / Wartung (500/503)
      content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } }
  schemas:
    Problem:
      type: object
      required: [type, title, status, code]
      properties:
        type: { type: string, format: uri }
        title: { type: string }
        status: { type: integer }
        detail: { type: string }
        code:
          type: string
          enum: [token_expired, token_invalid, insufficient_scope, invalid_parameter, not_found, rate_limited,
                 client_outdated, mc_uuid_taken, mc_proof_failed, mc_not_linked,
                 challenge_expired, not_registered, not_approved, event_not_started,
                 event_ended, maintenance, internal]
    Health:
      type: object
      required: [status, minClientVersion]
      properties:
        status: { type: string, enum: [ok, degraded, maintenance] }
        minClientVersion: { type: string, description: 'Mindestversion für X-Twogg-Client=pinecone, z. B. "0.9.6"' }
        message: { type: [string, 'null'], description: Wartungshinweis in Du-Form }
    Me:
      type: object
      required: [id, username]
      properties:
        id: { type: string }
        username: { type: string }
        avatarUrl: { type: [string, 'null'], format: uri }
        minecraft:
          type: [object, 'null']
          required: [uuid, name, verifiedAt]
          properties:
            uuid: { type: string, pattern: '^[0-9a-f]{32}$' }
            name: { type: string }
            verifiedAt: { type: string, format: date-time }
    McChallenge:
      type: object
      required: [serverId, expiresAt]
      properties:
        serverId: { type: string, description: 'zufällig, max. 20 Zeichen [ungeprüft: Längengrenze]' }
        expiresAt: { type: string, format: date-time }
    McVerifyRequest:
      type: object
      required: [serverId, username]
      properties:
        serverId: { type: string }
        username: { type: string, description: aktueller Minecraft-Name }
    Event:
      type: object
      required: [id, title, eventType, status]
      properties:
        id: { type: string }
        slug: { type: string, description: 'z. B. "revival-avalon" für Links' }
        title: { type: string, description: 'ohne Emojis (Pinecone filtert zusätzlich)' }
        summary: { type: [string, 'null'], description: 'ein Absatz, Klartext' }
        eventType: { type: string, enum: [STANDARD, TEAM, ROLEPLAY] }
        status: { type: string, enum: [announced, registration_open, registration_closed, running, ended, cancelled] }
        eventDate: { type: [string, 'null'], format: date-time, description: 'null = Termin folgt' }
        endDate: { type: [string, 'null'], format: date-time }
        registrationDeadline: { type: [string, 'null'], format: date-time }
        requiresApplication: { type: boolean }
        minecraftVersion: { type: [string, 'null'] }
        maxPlayers: { type: [integer, 'null'] }
        teamSize: { type: [integer, 'null'] }
        guildOptions:
          type: array
          items: { $ref: '#/components/schemas/Guild' }
          description: 'z. B. Die Treue Hand, Der Phönixbund; leer = keine Gilden'
        logoUrl: { type: [string, 'null'], format: uri }
        bannerUrl: { type: [string, 'null'], format: uri }
        pageUrl: { type: string, format: uri, description: Event-Seite auf twogg.app }
        pineconePackId: { type: [string, 'null'], description: 'optional, siehe Q-02' }
    Guild:
      type: object
      required: [id, name]
      properties: { id: { type: string }, name: { type: string } }
    EventPage:
      type: object
      required: [items]
      properties:
        items: { type: array, items: { $ref: '#/components/schemas/Event' } }
        nextCursor: { type: [string, 'null'] }
    Participation:
      type: object
      required: [eventId, status]
      properties:
        eventId: { type: string }
        status:
          type: string
          enum: [registered, pre_registered, invited, application_pending,
                 application_rejected, waitlisted, cancelled]
          description: 'Abbildung aus den heutigen registrations/teams/preRegistrations/teamInvites, siehe Q-03'
        team: { type: [object, 'null'], properties: { id: { type: string }, name: { type: string }, role: { type: string, enum: [leader, member] } } }
        guild: { oneOf: [ { $ref: '#/components/schemas/Guild' }, { type: 'null' } ] }
        updatedAt: { type: string, format: date-time }
    ParticipationList:
      type: object
      required: [items]
      properties: { items: { type: array, items: { $ref: '#/components/schemas/Participation' } } }
    JoinInfo:
      type: object
      required: [approved]
      properties:
        approved: { type: boolean }
        serverAddress: { type: [string, 'null'], description: 'host[:port], nur wenn approved' }
        reason: { type: [string, 'null'], enum: [not_registered, application_pending, not_approved, event_not_started, event_ended, null] }
        opensAt: { type: [string, 'null'], format: date-time, description: ab wann beitreten möglich ist }
    Community:
      type: object
      required: [applicationsOpen, links]
      properties:
        applicationsOpen: { type: boolean }
        applicationsUrl: { type: [string, 'null'], format: uri }
        links:
          type: object
          properties:
            discord: { type: string, format: uri, description: 'https://twogg.chat' }
            lookingForGroup: { type: string, format: uri }
            events: { type: string, format: uri }
paths:
  /health:
    get:
      security: []
      summary: Erreichbarkeit, Mindestversion
      responses:
        '200': { description: OK, content: { application/json: { schema: { $ref: '#/components/schemas/Health' } } } }
        '503': { $ref: '#/components/responses/ServerError' }
  /me:
    get:
      summary: Eigenes TwoGG-Profil
      security: [ { oauth: [profile:read] } ]
      responses:
        '200': { description: OK, content: { application/json: { schema: { $ref: '#/components/schemas/Me' } } } }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '426': { $ref: '#/components/responses/Outdated' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /me/minecraft/challenge:
    post:
      summary: serverId für den Mojang-Besitzbeweis
      security: [ { oauth: [minecraft:link] } ]
      requestBody: { required: false, content: { application/json: { schema: { type: object, maxProperties: 0 } } } }
      responses:
        '200': { description: OK, content: { application/json: { schema: { $ref: '#/components/schemas/McChallenge' } } } }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '403': { $ref: '#/components/responses/Forbidden' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /me/minecraft/verify:
    post:
      summary: Besitzbeweis prüfen (hasJoined) und UUID verknüpfen
      security: [ { oauth: [minecraft:link] } ]
      requestBody: { required: true, content: { application/json: { schema: { $ref: '#/components/schemas/McVerifyRequest' } } } }
      responses:
        '200': { description: verknüpft, content: { application/json: { schema: { $ref: '#/components/schemas/Me' } } } }
        '400': { description: 'challenge_expired | mc_proof_failed', content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } } }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '409': { description: mc_uuid_taken, content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } } }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /me/minecraft:
    delete:
      summary: Verknüpfung der Minecraft-UUID lösen
      security: [ { oauth: [minecraft:link] } ]
      responses:
        '204': { description: gelöst }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '404': { description: mc_not_linked, content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } } }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /events:
    get:
      summary: Events (öffentlich lesbar)
      security: [ {}, { oauth: [events:read] } ]
      parameters:
        - { $ref: '#/components/parameters/IfNoneMatch' }
        - { name: status, in: query, schema: { type: string, enum: [upcoming, past, all], default: all } }
        - { name: limit, in: query, schema: { type: integer, minimum: 1, maximum: 100, default: 50 } }
        - { name: cursor, in: query, schema: { type: string } }
      responses:
        '200': { description: OK, headers: { ETag: { $ref: '#/components/headers/ETag' } }, content: { application/json: { schema: { $ref: '#/components/schemas/EventPage' } } } }
        '304': { $ref: '#/components/responses/NotModified' }
        '400': { description: invalid_parameter, content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } } }
        '426': { $ref: '#/components/responses/Outdated' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /events/{eventId}:
    get:
      summary: Ein Event
      security: [ {}, { oauth: [events:read] } ]
      parameters: [ { $ref: '#/components/parameters/EventId' }, { $ref: '#/components/parameters/IfNoneMatch' } ]
      responses:
        '200': { description: OK, headers: { ETag: { $ref: '#/components/headers/ETag' } }, content: { application/json: { schema: { $ref: '#/components/schemas/Event' } } } }
        '304': { $ref: '#/components/responses/NotModified' }
        '404': { $ref: '#/components/responses/NotFound' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /me/participations:
    get:
      summary: Eigene Teilnahmen (Anmeldung, Team, Gilde)
      security: [ { oauth: [participation:read] } ]
      parameters: [ { $ref: '#/components/parameters/IfNoneMatch' } ]
      responses:
        '200': { description: OK, headers: { ETag: { $ref: '#/components/headers/ETag' } }, content: { application/json: { schema: { $ref: '#/components/schemas/ParticipationList' } } } }
        '304': { $ref: '#/components/responses/NotModified' }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '403': { $ref: '#/components/responses/Forbidden' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /events/{eventId}/join:
    get:
      summary: Freigabe und Server-Adresse für den Beitritt
      security: [ { oauth: [participation:read] } ]
      parameters: [ { $ref: '#/components/parameters/EventId' } ]
      responses:
        '200': { description: 'OK (auch bei approved=false, dann mit reason)', content: { application/json: { schema: { $ref: '#/components/schemas/JoinInfo' } } } }
        '401': { $ref: '#/components/responses/Unauthorized' }
        '404': { $ref: '#/components/responses/NotFound' }
        '409': { description: 'mc_not_linked (keine Minecraft-UUID verknüpft)', content: { application/problem+json: { schema: { $ref: '#/components/schemas/Problem' } } } }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
  /community:
    get:
      security: []
      summary: Bewerbungsphase und Community-Links
      responses:
        '200': { description: OK, content: { application/json: { schema: { $ref: '#/components/schemas/Community' } } } }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '5XX': { $ref: '#/components/responses/ServerError' }
```

### 4.1 Übersicht der `/api/v1`-Endpunkte

Für GET-Endpunkte ohne Parameter ist das Request-Schema „kein Body, nur Kopfzeilen“.

| Endpunkt | Request | Response | Fehlercodes (HTTP · `code`) |
|---|---|---|---|
| `GET /health` | kein Body | `200 Health` | `503 maintenance` |
| `GET /me` | kein Body, Bearer `profile:read` | `200 Me` | `401 token_expired/token_invalid`, `426 client_outdated`, `429 rate_limited`, `5xx internal` |
| `POST /me/minecraft/challenge` | leeres Objekt, Bearer `minecraft:link` | `200 McChallenge` | `401`, `403 insufficient_scope`, `429`, `5xx` |
| `POST /me/minecraft/verify` | `McVerifyRequest` | `200 Me` | `400 challenge_expired/mc_proof_failed`, `401`, `409 mc_uuid_taken`, `429`, `5xx` |
| `DELETE /me/minecraft` | kein Body | `204` | `401`, `404 mc_not_linked`, `5xx` |
| `GET /events` | Query `status`, `limit`, `cursor`; `If-None-Match` | `200 EventPage` + ETag, `304` | `400 invalid_parameter`, `426`, `429`, `5xx` |
| `GET /events/{eventId}` | Pfad `eventId`; `If-None-Match` | `200 Event` + ETag, `304` | `404 not_found`, `429`, `5xx` |
| `GET /me/participations` | `If-None-Match`, Bearer `participation:read` | `200 ParticipationList` + ETag, `304` | `401`, `403`, `429`, `5xx` |
| `GET /events/{eventId}/join` | Pfad `eventId`, Bearer | `200 JoinInfo` | `401`, `404 not_found`, `409 mc_not_linked`, `429`, `5xx` |
| `GET /community` | kein Body | `200 Community` | `429`, `5xx` |

### 4.2 OAuth-Endpunkte (außerhalb `/api/v1`, Host twogg.app)

| Endpunkt | Request | Response | Fehler |
|---|---|---|---|
| `GET /oauth/authorize` | Query: `response_type=code`, `client_id=pinecone`, `redirect_uri=http://127.0.0.1:<port>/callback`, `scope`, `state` (≥ 128 bit), `code_challenge` (Base64url SHA-256), `code_challenge_method=S256` | `302` auf `redirect_uri?code=<einmalig, 60 s gültig>&state=<state>` | `302` mit `error=access_denied` / `invalid_request` / `invalid_scope` / `server_error` und `state`. Bei ungültiger `client_id` oder `redirect_uri`: **keine** Umleitung, Fehlerseite (RFC 6749 §4.1.2.1 [ungeprüft: RFC nicht einzeln geprüft]) |
| `POST /oauth/token` (`application/x-www-form-urlencoded`) | a) `grant_type=authorization_code`, `code`, `redirect_uri`, `client_id`, `code_verifier`<br>b) `grant_type=refresh_token`, `refresh_token`, `client_id` | `200` `{ "access_token", "token_type": "Bearer", "expires_in", "refresh_token", "scope" }` | `400` `{ "error": "invalid_grant" \| "invalid_request" \| "unsupported_grant_type" \| "invalid_client" }`, `429`, `5xx` |
| `POST /oauth/revoke` | `token`, `token_type_hint=refresh_token`, `client_id` | `200` (auch bei unbekanntem Token) | `400 invalid_request`, `429`, `5xx` |

## 5. Statusänderungen: Polling statt Push

**Entscheidung: Bedingtes Polling mit ETag.** SSE und WebSocket kommen vorerst nicht.

| Kriterium | Polling + ETag | SSE | WebSocket |
|---|---|---|---|
| Aufwand twogg.app | gering (ETag an zwei Endpunkten) | mittel: dauerhafte Verbindungen, Proxy-Puffer abschalten | hoch |
| Betrieb hinter Cloudflare und Caddy (beobachtet) | unkritisch | lange Verbindungen müssen Proxy-Timeouts überstehen [ungeprüft, U-09] | wie SSE |
| Bei Serverless-Hosting | unkritisch | auf Vercel durch die maximale Funktionsdauer begrenzt, der Client muss neu verbinden (Vercel-Doku, Stand 01.10.2026). Nur relevant, falls twogg.app dorthin zieht | WebSockets auf Vercel nur Beta |
| Aktualität | Intervall | sofort | sofort |
| Bedarf | Status ändert sich selten (Anmeldung, Freigabe) | – | – |

**Intervalle:**

| Situation | Intervall |
|---|---|
| App im Hintergrund | 15 min |
| App im Vordergrund | 5 min |
| Events-Seite offen oder ein angemeldetes Event startet in < 2 h | 60 s |
| nach Rückkehr aus dem Browser (Fokus) | sofort |

- `If-None-Match` sorgt dafür, dass unveränderte Daten als `304` ohne Body zurückkommen.
- Bei `429` oder `5xx` verdoppelt sich das Intervall bis maximal 30 min.
- **Spätere Option:** `GET /api/v1/me/stream` (SSE, Ereignisse `participation.updated`, `event.updated`), wenn das Team unmittelbare Freigaben braucht (Q-04).

## 6. Event-Pack-Manifest

**Ziel:** Pinecone installiert nur echte TwoGG-Packs. Die Echtheit hängt nicht davon ab, dass der Pinecone-Server sicher ist.

### 6.1 Format

Datei `manifest.json`, liegt neben dem `.mrpack` auf dem Pinecone-Server:

```json
{
  "schema": "pinecone-eventpack/1",
  "packId": "avalon",
  "version": "1.4.0",
  "channel": "release",
  "eventId": "[TWOGG-EVENT-ID]",
  "title": "REVIVAL: Avalon",
  "minecraft": "1.21.11",
  "loader": { "kind": "fabric", "version": "[LOADER-VERSION]" },
  "minLauncherVersion": "0.10.0",
  "required": true,
  "allowExtraMods": false,
  "recommendedMemoryMb": 6144,
  "mrpack": {
    "url": "https://pinecone.twogg.app/api/v1/packs/avalon/versions/1.4.0/download",
    "size": 123456789,
    "sha512": "<hex>"
  },
  "locked": [
    { "path": "mods/[DATEI].jar", "sha512": "<hex>" },
    { "path": "config/[DATEI].toml", "sha512": "<hex>" }
  ],
  "publishedAt": "2026-10-12T18:00:00Z",
  "keyId": "twogg-2026"
}
```

| Feld | Pflicht | Bedeutung |
|---|---|---|
| `schema` | ja | Formatversion. Pinecone lehnt unbekannte Hauptversionen ab |
| `packId`, `version`, `channel` | ja | Identität; `channel` ist einer von `release`, `beta`, `alpha` (wie im Server) |
| `eventId` | ja | twogg.app-Event. Pinecone prüft, dass es zum gewählten Event passt |
| `minecraft`, `loader` | ja | muss mit `modrinth.index.json` im `.mrpack` übereinstimmen |
| `minLauncherVersion` | ja | ältere Launcher installieren nicht (AK-10.3) |
| `required` | ja | Pflicht-Update vor dem Start (F-12) |
| `allowExtraMods` | ja | Zusatz-Mods erlaubt (F-11) |
| `recommendedMemoryMb` | nein | Hinweis in der Instanz (AK-32.4) |
| `mrpack.url`, `size`, `sha512` | ja | Download und Prüfung. Die URL liefert einen HMAC-Link (Bestand) |
| `locked[]` | ja | Pflichtdateien nach der Installation, mit SHA-512 |
| `keyId` | ja | welcher öffentliche Schlüssel prüft |

### 6.2 Signatur

- **Verfahren:**
  - Ed25519 über die kanonische Form des Manifests (JSON Canonicalization Scheme, RFC 8785 [ungeprüft: RFC nicht einzeln geprüft]).
  - Die Signatur liegt getrennt in `manifest.json.sig` (Base64, 64 Byte).
  - Abruf: `GET /api/v1/packs/{id}/versions/{v}/manifest` liefert `{ "manifest": {…}, "signature": "<base64>" }`.
- **Schlüssel:**
  - Das Schlüsselpaar wird offline erzeugt. Der private Schlüssel liegt verschlüsselt bei [VERANTWORTLICHE PERSON] im Orga-Team und auf einem Sicherungsträger.
  - Der öffentliche Schlüssel ist in den Launcher kompiliert (`EVENTPACK_KEYS: {keyId → pubkey}`).
- **Signieren:**
  - Ein CLI-Befehl `pinecone-cli eventpack sign <mrpack> --event <id> …` (neu) erzeugt Manifest und Signatur aus dem `.mrpack`, inklusive `locked` aus allen Dateien des Packs.
  - Beides wird im Admin hochgeladen. Der Server prüft beim Upload die Signatur gegen dieselben öffentlichen Schlüssel und weist ungültige ab.
- **Schlüsselwechsel:**
  1. Neuer `keyId` wird mit einem Launcher-Update ausgeliefert.
  2. Ab dann wird mit dem neuen Schlüssel signiert.
  3. Der alte Schlüssel wird nach Ablauf der Mindestversion entfernt.
  - Ein kompromittierter Schlüssel wird mit einem Update auf eine Sperrliste gesetzt und `minLauncherVersion` angehoben.
- **Prüfung im Launcher (F-10):**
  1. Signatur prüfen.
  2. `schema` prüfen.
  3. `eventId` prüfen.
  4. `minLauncherVersion` prüfen.
  5. Download.
  6. Größe und SHA-512 prüfen.
  7. `modrinth.index.json` gegen `minecraft`/`loader` prüfen.
  8. Installieren.
  9. Jede Datei aus `locked` per SHA-512 prüfen.
- Schlägt ein Schritt fehl, wird nichts installiert (AK-10.1, AK-10.2).

### 6.3 Endpunkte am Pinecone-Server (neu)

Sie liegen nicht bei twogg.app, gehören aber zum Vertrag des Launchers. Fehler folgen dem Format des Pinecone-Servers: `{"error": "<text>"}` mit HTTP-Status, wie heute in `split/server/src/api.rs`.

| Endpunkt | Request | Response | Fehler |
|---|---|---|---|
| `GET /api/v1/packs/{id}/versions/{v}/manifest` | Pfad `id`, `v`; Kopfzeile `X-Pinecone-Client: pinecone/*` | `200 { "manifest": <Manifest §6.1>, "signature": "<base64>" }` | `403` ohne Client-Kopfzeile, `404` Pack/Version unbekannt oder ohne Manifest, `429` Rate-Limit (wie Download-Links), `5xx` |
| `GET /api/v1/packs/{id}/manifest/latest?channel=release` | Pfad `id`, Query `channel` (`release`/`beta`/`alpha`) | `200` wie oben für die neueste Version des Kanals | `403`, `404`, `429`, `5xx` |
| `DELETE /api/v1/me` (umgesetzt, Server-PR #4) | Bearer Freunde-Sitzung | `204` – Spielerprofil, Sitzungen, Anfragen, Freundschaften gelöscht | `401` Sitzung ungültig, `5xx` |
| `GET\|POST /api/v1/curseforge/v1/...` (umgesetzt, Server-PR #4) | Kopfzeile `X-Pinecone-Client`; `GET mods/search` (Query wie CurseForge, `gameId` fest 432), `GET mods/{id}/files`, `POST mods/files` `{fileIds}`, `POST mods` `{modIds}` (je ≤ 500 IDs) | Antwort von CurseForge unverändert, `Cache-Control: no-store` | `400` zu viele IDs, `403` ohne Client-Kopfzeile, `429` mehr als 240/min je IP, `502` CurseForge lehnt den Schlüssel ab oder ist nicht erreichbar, `503` kein Schlüssel eingerichtet |
| `GET /api/v1/info` (erweitert, Server-PR #4) | – | zusätzlich `curseforge: bool` und `legal: {imprintUrl, privacyUrl, contactEmail}` | `5xx` |
| `GET\|PUT /api/v1/lobby/{eventId}` (v2) | Bearer Freunde-Sitzung; `PUT {ready: bool}`; `GET ?members=<uuid,…>` (≤ 100) | `GET 200 {members: [{uuid, state: offline\|launcher\|ready\|playing}]}`, `PUT 204` | `400` zu viele UUIDs, `401`, `429` mehr als 1 Abruf je 30 s | 
| `POST /api/v1/invites` (v2) | Bearer; `{to: uuid, server: {address}, pack?: {link}}` | `201 {id, expiresAt}`; Empfänger erhält sie in `GET /api/v1/friends` unter `invites[]` | `400`, `401`, `403` kein Freund, `429` > 10/min |
| `POST /api/v1/invites/{id}/answer` (v2) | Bearer; `{accept: bool}` | `204` | `401`, `404` abgelaufen |
| `GET /api/v1/friends/code` · `POST /api/v1/friends/code` (v2) | Bearer | `200 {code}` (POST erzeugt neu) | `401` |
| `POST /api/v1/friends/requests` (erweitert, v2) | zusätzlich `{code}` statt `{name}` | wie bisher | `404` Code unbekannt, `429` > 10 Fehlversuche/h je IP |
| `POST /api/v1/sync-shares` · `GET /api/v1/sync-shares/{code}` (v2) | Bearer zum Anlegen; Body Sync-Pack ohne eigene Dateien (≤ 64 KB) | `201 {code, expiresAt}` · `200 <Sync-Pack>` | `400` zu groß/ungültig, `401`, `404` abgelaufen |
| `POST /api/v1/gallery` · `DELETE /api/v1/gallery/{id}` · `GET /api/v1/events/{id}/gallery` (v2) | Bearer; Multipart `image` (PNG/JPEG ≤ 8 MB), `eventId` | `201 {id, status: pending}` · `204` · `200 {images: [{id, url, mcName}]}` (nur freigegebene) | `400` Format, `401`, `413`, `429` > 20 je Event und Spieler |
| `GET /api/v1/info` (erweitert, v2) | – | zusätzlich `banner?: {id, level, text, link?, until}` und `minLauncherVersion?` | `5xx` |
| `GET /api/v1/packs/{id}` (erweitert, v2) | – | Versionen tragen `withdrawn?: {reason}`; zurückgezogene fehlen in Listen und `latest` | wie bisher |
| `POST /api/admin/versions/{id}/server-pack` · `POST /api/admin/versions/{id}/withdraw` (v2) | Admin | `200` ZIP · `204` | `401`, `403` Rolle, `404` |
| `/api/admin/accounts`, `/api/admin/audit` (v2) | Admin-Rolle | CRUD Konten · Liste Protokoll | `401`, `403` |
| `POST /api/admin/packs/{id}/versions` (erweitert) | Multipart: `file` (.mrpack), `manifest` (JSON), `signature` (Base64), `channel` | `201` Version | `400` Manifest passt nicht zum .mrpack (Version, Loader, SHA-512), `400` Signatur ungültig, `401`, `413` zu groß |

## 7. Was auf twogg.app-Seite neu gebaut werden muss

Laut Aufgabe ist twogg.app eine Next.js-/Node-App. Die genaue Struktur ist ohne Repo nicht prüfbar (U-09).

| # | Baustein | Inhalt | Funktion |
|---|---|---|---|
| T-1 | OAuth 2.0 Authorization Server | `/oauth/authorize` mit Login-Weiterleitung und Zustimmungsseite, `/oauth/token`, `/oauth/revoke`. Client-Tabelle (`pinecone`), Code-Speicher (60 s, einmalig), Refresh-Token-Speicher (gehasht, rotierend, widerrufbar). Nur PKCE S256, Loopback-Redirects mit beliebigem Port | F-07 |
| T-2 | Bearer-Auth für `/api/v1` | Token prüfen, Scopes prüfen, `problem+json`-Fehler | alle |
| T-3 | Minecraft-Verknüpfung | `challenge` (zufällige `serverId`, 2 min gültig), `verify` (Mojang `hasJoined`), Eindeutigkeit der UUID, `DELETE`. Datenfelder `mcUuid`, `mcName`, `mcVerifiedAt` am Nutzer | F-07 |
| T-4 | `/api/v1/events` und `/events/{id}` | Feldabbildung aus dem bestehenden `/events`, dazu `status`, `guildOptions`, `pageUrl`, ETag, Paginierung | F-08 |
| T-5 | `/api/v1/me/participations` | ein konsolidierter Status je Event aus Registrierungen, Teams, Vormerkungen und Einladungen. Gilde, ETag | F-09 |
| T-6 | `/api/v1/events/{id}/join` | heutige Logik plus `reason`, `opensAt`, Prüfung der verknüpften UUID | F-13 |
| T-7 | `/api/v1/community` | Bewerbungsphase aus der twogg.app-Bewerbungsverwaltung, Links | F-18 |
| T-8 | `/api/v1/health` | `minClientVersion` je Client-Typ (`pinecone` getrennt von `desktop`) | F-07, F-34 |
| T-9 | Rate-Limiting und Logging | je Token und IP. Logs ohne Tokens | alle |
| T-10 | Kontoeinstellungen auf twogg.app | Seite „Verbundene Apps“ mit Pinecone, Zeitpunkt, „Zugriff entziehen“. Minecraft-Verknüpfung anzeigen und lösen | F-07, DSGVO |
| T-11 | Datenschutzerklärung | Abschnitt Pinecone (§9) | DSGVO |
| T-12 (v2) | `GET /api/v1/events/{id}/results` | Veröffentlichte Ergebnisse: `published`, `teams[] {teamId, name, rank, score}`, `guilds[] {…}`, optional `me {rank, score, stats}`. Scope `events:read`, ETag. `404` bis zur Veröffentlichung | F-43, F-47 |
| T-13 (v2) | `GET /api/v1/guilds/{id}` | Name, Farbe/Wappen, Mitglieder `{mcUuid, mcName, displayName}`, Events der Gilde. Nur für Mitglieder (`403` sonst) | F-45 |
| T-14 (v2) | `GET /api/v1/me/participations` erweitert | Je Teilnahme die Teammitglieder mit `mcUuid` (nur wenn verknüpft), damit die Lobby (F-44) sie der Präsenz des Pinecone-Servers zuordnen kann | F-44 |

Die Bibliothekswahl für T-1 trifft das twogg.app-Team. Kandidaten wären etwa `oidc-provider` für Node [ungeprüft: Eignung nicht geprüft].

## 8. Verhalten, wenn twogg.app nicht erreichbar ist

| Situation | Pinecone zeigt / tut | Was weiter geht |
|---|---|---|
| Netz weg oder Zeitüberschreitung (15 s) | Chip „Offline · Stand von <Zeit>“, Events und Status aus dem Cache (`02` §9.3) | Starten aller installierten Instanzen inkl. Event-Instanzen; Countdown aus Cache |
| `503 maintenance` | Tafel mit `message` aus `/health`, sonst „twogg.app wird gerade gewartet“ | wie oben |
| `5xx` andere | „twogg.app antwortet gerade nicht“, erneuter Versuch nach dem Backoff (§5) | wie oben |
| `401` nach gescheitertem Refresh | Verknüpfung getrennt, „Verbindung zu twogg.app erneuern“ | Events öffentlich lesbar. Status und Beitritt nicht |
| `426 client_outdated` | „Bitte aktualisiere Pinecone“, Knopf zu F-34 | lokale Funktionen |
| `GET /events/{id}/join` nicht erreichbar | „Server beitreten“ deaktiviert: „Freigabe kann gerade nicht geprüft werden“ | Spielen der Event-Instanz ohne Server-Beitritt; Adresse über Discord |
| Pinecone-Server down (Packs) | „Event-Pack kann gerade nicht geladen werden“ | Installierte Event-Instanzen starten, Integrität aus dem lokalen Cache |

**Grundsätze:**

- Pinecone speichert die **Server-Adresse nie**, weil der Zugang bei twogg.app liegt.
- Der Lese-Cache gilt nicht als Freigabe.
- Ohne twogg.app bleibt Pinecone ein vollständiger Launcher.

## 9. Datenschutz (DSGVO)

### 9.1 Daten

| Datum | Wo | Zweck | Rechtsgrundlage (Vorschlag, juristisch zu prüfen) | Löschung |
|---|---|---|---|---|
| Microsoft-/Minecraft-Tokens | Gerät, OS-Schlüsselbund | Spielstart | Art. 6 (1) b DSGVO (Nutzung des Launchers) | Konto entfernen; Deinstallation (Schlüsselbund-Einträge werden beim Entfernen gelöscht, bei Deinstallation [ungeprüft, ob der Installer sie entfernt]) |
| Kontoliste (UUID, Name, Skin-URL) | Gerät `accounts.json` | Anzeige | Art. 6 (1) b | Konto entfernen |
| twogg.app-Refresh-Token | Gerät, Schlüsselbund | Verknüpfung | Art. 6 (1) b | Trennen (+ Revoke bei twogg.app) |
| Events/Teilnahmen-Cache | Gerät `cache/twogg-app.json` | Offline-Anzeige | Art. 6 (1) b | Trennen, Kontowechsel, Deinstallation |
| TwoGG-Konto ↔ Minecraft-UUID, Name, Prüfzeitpunkt | twogg.app | Eventzugang, Freigabe | Art. 6 (1) b (Teilnahme an Events) | Lösen in Pinecone oder twogg.app; Kontolöschung auf twogg.app |
| OAuth-Refresh-Tokens (gehasht), Zustimmungen | twogg.app | Verknüpfung | Art. 6 (1) b | Widerruf, nach Ablauf, Kontolöschung |
| Server-Logs mit IP | twogg.app, Pinecone-Server | Sicherheit, Rate-Limit | Art. 6 (1) f (berechtigtes Interesse) | [AUFBEWAHRUNGSFRIST], Vorschlag 14 Tage |
| Freunde, Präsenz, letzte Aktivität | Pinecone-Server (SQLite) | Freundesliste | Art. 6 (1) b, Sichtbarkeit abschaltbar | „Meine Freunde-Daten löschen“ unter Einstellungen › Konten (`DELETE /api/v1/me`, umgesetzt); automatische Löschung inaktiver Profile nach 365 Tagen (`PINECONE_PLAYER_RETENTION_DAYS`, umgesetzt) |
| v2: Bereit-Status (F-44), Einladungen (F-56), Freundescode (F-57) | Pinecone-Server | Lobby, Einladungen, Freunde | Art. 6 (1) b | Bereit-Status ≤ 2 h, Einladungen ≤ 15 min, Code bis zum Neu-Erzeugen; „Meine Freunde-Daten löschen“ löscht alles |
| v2: Galerie-Bilder mit Minecraft-Name (F-46) | Pinecone-Server | Event-Galerie | Art. 6 (1) a (Einwilligung je Upload) [PRÜFEN] | Löschen durch Spieler jederzeit, Orga-Team kann ablehnen |
| v2: Geteilte Sync-Packs (F-58) | Pinecone-Server | Teilen-Link | Art. 6 (1) b | 30 Tage |
| v2: Nutzungsstatistik (F-59) | Pinecone-Server | Admin-Dashboard | keine personenbezogenen Daten (nur Tageszähler) [PRÜFEN] | 2 Jahre |
| CurseForge-Anfragen (Suchbegriffe, IDs) | Pinecone-Server, nur durchgereicht | CurseForge-Inhalte | Art. 6 (1) b | werden nicht gespeichert; IP nur im Arbeitsspeicher für die Ratenbegrenzung |
| Spielzeit, Instanzen, Logs | nur Gerät | Funktionen | – (verlässt das Gerät nicht) | Deinstallation / Ordner löschen |
| Diagnose-ZIP | Gerät, vom Nutzer weitergegeben | Support | Einwilligung durch aktives Versenden | Nutzer |

- **Keine Telemetrie** und keine Analyse-Dienste. Schriften sind lokal gebündelt, damit beim Start keine Verbindung zu Google Fonts entsteht.

### 9.2 Pflichten

- **Informationspflicht** (Art. 13): Pinecone verlinkt in Einstellungen › Allgemein („Über Pinecone“, umgesetzt) auf die Datenschutzerklärung, deren Adresse im Admin gepflegt wird. Ein Entwurf liegt in `anhang-datenschutzerklaerung-entwurf.md`. Sie beschreibt Launcher, Pinecone-Server und den twogg.app-Anteil. Hilfe: DSK-Kurzpapier Nr. 10.
- **Recht auf Löschung** (Art. 17):
  - Lokale Daten löscht der Nutzer selbst, mit dem neuen Knopf „Alle lokalen Daten löschen“ unter Einstellungen › Allgemein.
  - Server-Daten löscht er per Knopf (Pinecone-Server) bzw. über die twogg.app-Kontolöschung.
  - Hilfe: DSK-Kurzpapier Nr. 11.
- **Minderjährige:** Bei Art. 6 (1) f überwiegen laut Normtext die Interessen von Kindern eher. Da viele Minecraft-Spieler minderjährig sind, stützt sich der Plan überall außer bei Sicherheitslogs auf Art. 6 (1) b. Die Frage braucht eine rechtliche Prüfung (Q-08).
- **Verantwortlicher:** [VERANTWORTLICHE STELLE TWOGG] (Impressum, Kontakt).

Quellen: DSGVO-Text (Art. 6, 13, 17) über gdpr-info.eu (Spiegel, EUR-Lex war nicht abrufbar); DSK-Kurzpapiere Nr. 10 und 11 (datenschutzkonferenz-online.de, als „in Überarbeitung“ gekennzeichnet).
