# Phase 5: Pro Features PRD

**Bead:** bd-66j
**Type:** Epic
**Status:** In Progress
**Created:** 2026-02-05

## Bead Metadata

```yaml
depends_on: [bd-cdx] # Phase 4 Timeline Editing
parallel: false
conflicts_with: []
blocks: []
estimated_hours: 80
```

## Overview

Phase 5 introduces the Pro tier features that differentiate Frame's business model: cloud sync for backup and access anywhere, shareable links for easy distribution, and team workspaces for collaborative workflows. This phase establishes the backend infrastructure and client integrations.

## Goals

1. **Cloud Infrastructure**: Set up Supabase (auth, database) + Cloudflare (R2 storage, Workers)
2. **User Authentication**: Sign up, login, session management
3. **Video Upload**: Upload recordings to cloud storage with progress tracking
4. **Shareable Links**: Generate public/private links for recordings
5. **Team Workspaces**: Basic team management (invite, roles, shared library)
6. **License Verification**: Check Pro tier status, enable/disable features

## Non-Goals

- AI features (auto-zoom suggestions, silence removal) - Future phase
- Advanced team features (comments, approval workflows) - Future phase
- Custom export presets - Future phase
- Mobile apps - Out of scope

## Technical Approach

### Backend Stack

- **Supabase**: PostgreSQL database, Auth, Row Level Security
- **Cloudflare R2**: Video file storage (S3-compatible, cheap egress)
- **Cloudflare Workers**: Edge functions for link generation, presigned URLs

### Client Integration

- New `packages/cloud/` Rust crate for API client
- OAuth/session management with secure token storage (macOS Keychain)
- Background upload queue with retry logic
- Offline-first: local projects work without internet

### Database Schema

```sql
-- Users (managed by Supabase Auth)
-- profiles table for extended user data
profiles (
  id uuid references auth.users primary key,
  display_name text,
  avatar_url text,
  tier text default 'free', -- 'free' | 'pro' | 'enterprise'
  created_at timestamptz default now()
)

-- Teams
teams (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  owner_id uuid references profiles(id),
  created_at timestamptz default now()
)

-- Team memberships
team_members (
  team_id uuid references teams(id),
  user_id uuid references profiles(id),
  role text default 'member', -- 'owner' | 'admin' | 'member'
  joined_at timestamptz default now(),
  primary key (team_id, user_id)
)

-- Recordings (metadata only, files in R2)
recordings (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid references profiles(id),
  team_id uuid references teams(id), -- nullable for personal
  title text not null,
  description text,
  duration_ms bigint,
  file_size_bytes bigint,
  r2_key text not null, -- path in R2 bucket
  thumbnail_key text,
  share_token text unique, -- for shareable links
  is_public boolean default false,
  created_at timestamptz default now(),
  updated_at timestamptz default now()
)
```

## Tasks

### Infrastructure Setup [infra]

Set up Supabase project and Cloudflare resources.

**Verification:**

- Supabase project accessible with API keys
- R2 bucket created with CORS configured
- Worker deployed for presigned URL generation
- Database migrations applied successfully

**Metadata:**

```yaml
depends_on: []
parallel: false
conflicts_with: []
files: [infra/, supabase/migrations/]
```

### Database Schema [db]

Create database tables with Row Level Security policies.

**Verification:**

- All tables created (profiles, teams, team_members, recordings)
- RLS policies enforce ownership/team access
- Indexes on frequently queried columns
- Test queries return expected results

**Metadata:**

```yaml
depends_on: [infra-1]
parallel: false
conflicts_with: []
files: [supabase/migrations/]
```

### Cloud Client Crate [core]

Create `packages/cloud/` Rust crate for API interactions.

**Verification:**

- `cargo build -p frame-cloud` succeeds
- Client can authenticate with Supabase
- Client can upload file to R2 via presigned URL
- Client can fetch recording metadata

**Metadata:**

```yaml
depends_on: [db-1]
parallel: true
conflicts_with: []
files: [packages/cloud/]
```

### Secure Token Storage [core]

Store auth tokens securely using macOS Keychain.

**Verification:**

- Tokens saved to Keychain on login
- Tokens retrieved on app launch
- Tokens cleared on logout
- No tokens in plaintext config files

**Metadata:**

```yaml
depends_on: [core-1]
parallel: true
conflicts_with: []
files: [packages/cloud/src/keychain.rs]
```

### Auth UI [ui]

Login/signup UI components in the desktop app.

**Verification:**

- Login form with email/password fields
- Signup form with validation
- "Forgot password" flow triggers email
- User avatar/name shown when logged in
- Logout button clears session

**Metadata:**

```yaml
depends_on: [core-1, core-2]
parallel: true
conflicts_with: []
files: [apps/desktop/src/ui/auth.rs, packages/ui-components/src/components/auth.rs]
```

### Upload Manager [core]

Background upload queue with progress and retry.

**Verification:**

- Upload starts when user clicks "Upload to Cloud"
- Progress percentage updates in UI
- Failed uploads retry automatically (3 attempts)
- Upload resumes after app restart
- Large files (>1GB) upload successfully

**Metadata:**

```yaml
depends_on: [core-1]
parallel: true
conflicts_with: []
files: [packages/cloud/src/upload.rs]
```

### Upload UI [ui]

Upload progress and status in the desktop app.

**Verification:**

- Progress bar shows upload percentage
- "Uploading" status visible in project list
- Cancel button stops upload
- Error state shown with retry option
- Success state shows "View online" link

**Metadata:**

```yaml
depends_on: [core-3, ui-1]
parallel: true
conflicts_with: []
files: [apps/desktop/src/ui/cloud.rs, packages/ui-components/src/components/upload.rs]
```

### Shareable Links Worker [infra]

Cloudflare Worker for generating and resolving share links.

**Verification:**

- POST /share creates new share token
- GET /s/{token} returns recording metadata
- GET /s/{token}/video streams video file
- Private links require auth header
- Expired tokens return 404

**Metadata:**

```yaml
depends_on: [infra-1, db-1]
parallel: true
conflicts_with: []
files: [workers/share/]
```

### Share UI [ui]

Share dialog in desktop app with copy-to-clipboard.

**Verification:**

- "Share" button opens dialog
- Toggle for public/private link
- Copy button copies link to clipboard
- Link preview shows shortened URL
- Revoke button invalidates link

**Metadata:**

```yaml
depends_on: [infra-2, core-1]
parallel: true
conflicts_with: []
files: [apps/desktop/src/ui/share.rs, packages/ui-components/src/components/share.rs]
```

### Team Management API [core]

CRUD operations for teams and memberships.

**Verification:**

- Create team with name
- Invite user by email
- Accept/decline invitation
- Remove member from team
- Transfer ownership
- Delete team (owner only)

**Metadata:**

```yaml
depends_on: [db-1, core-1]
parallel: true
conflicts_with: []
files: [packages/cloud/src/teams.rs]
```

### Team UI [ui]

Team management interface in settings.

**Verification:**

- Team list in settings sidebar
- Create team form
- Member list with roles
- Invite member dialog
- Role change dropdown (admin only)
- Leave team button

**Metadata:**

```yaml
depends_on: [core-5, ui-1]
parallel: true
conflicts_with: []
files: [apps/desktop/src/ui/teams.rs, packages/ui-components/src/components/teams.rs]
```

### License Verification [core]

Check user tier and enable/disable Pro features.

**Verification:**

- Free users see "Upgrade to Pro" prompts
- Pro features gated behind tier check
- License refreshes on app launch
- Offline grace period (7 days)
- Expired Pro reverts to Free features

**Metadata:**

```yaml
depends_on: [core-1]
parallel: true
conflicts_with: []
files: [packages/cloud/src/license.rs]
```

### Pro Feature Gates [desktop]

Integrate license checks into desktop app.

**Verification:**

- Cloud sync disabled for Free tier
- Share links limited to public for Free
- Team features disabled for Free
- Upgrade prompts show in appropriate places
- Pro badge shown in UI when active

**Metadata:**

```yaml
depends_on: [core-6, ui-2, ui-3, ui-4]
parallel: false
conflicts_with: []
files: [apps/desktop/src/app.rs, apps/desktop/src/ui/]
```

### Web Viewer [web]

SolidJS app for viewing shared recordings.

**Verification:**

- /s/{token} loads video player
- Player shows title, duration
- Video streams without download prompt
- Mobile-responsive layout
- 404 page for invalid tokens

**Metadata:**

```yaml
depends_on: [infra-2]
parallel: true
conflicts_with: []
files: [apps/web/]
```

### Documentation [docs]

Update AGENTS.md and user docs for Pro features.

**Verification:**

- AGENTS.md updated for packages/cloud
- User guide for cloud sync setup
- API documentation for Workers
- Team setup guide
- Troubleshooting section

**Metadata:**

```yaml
depends_on: [all above]
parallel: false
conflicts_with: []
files: [packages/cloud/AGENTS.md, docs/]
```

## Acceptance Criteria

1. User can sign up, login, and logout
2. User can upload recording to cloud with progress
3. User can generate shareable link (public or private)
4. User can create team and invite members
5. Free tier users see appropriate upgrade prompts
6. Pro features are gated behind license verification
7. Web viewer plays shared recordings
8. All verification steps pass
9. `cargo clippy --workspace -- -D warnings` passes
10. `cargo test --workspace` passes

## Out of Scope (Future Phases)

- Payment/billing integration (Stripe) - Phase 6
- AI features (auto-zoom, silence removal) - Phase 7
- Comments and approval workflows - Phase 7
- Analytics dashboard - Phase 7
- Custom domains for share links - Future
- SSO/SAML for enterprise - Future
