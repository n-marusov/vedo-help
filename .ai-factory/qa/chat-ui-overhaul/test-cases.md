# Test Cases: Chat UI Overhaul + Authentication

> Based on `change-summary.md` and `test-plan.md`
> Framework: Playwright (E2E) + Vitest (unit/component)

## Test Files

| File | Coverage | Related Tasks |
|------|----------|---------------|
| `e2e/avatar.spec.ts` | Avatar component — initials, colors, status, sizes | Task 1.2 |
| `e2e/message-bubble.spec.ts` | MessageBubble redesign — layout, markdown, sources, streaming/typing | Tasks 2.1, 2.2 |
| `e2e/chat-window.spec.ts` | ChatWindow layout — header, input, collection selector, animations | Tasks 3.2, 3.3 |
| `e2e/navigation.spec.ts` | Admin nav removal, routing, responsive layout | Tasks 3.1, 3.4 |
| `e2e/login.spec.ts` | Login page, social providers, auth guard, token persistence | Tasks 5.5, 5.6 |

## Test Case Summary

| ID | Name | Priority | Type |
|----|------|----------|------|
| **Avatar Component** | | | |
| TC-AVATAR-001 | User avatar renders with correct initials | High | Unit |
| TC-AVATAR-002 | Assistant avatar renders with correct initials | High | Unit |
| TC-AVATAR-003 | Consistent color assignment per identifier | Medium | Unit |
| TC-AVATAR-004 | Online status indicator visible | Medium | Unit |
| TC-AVATAR-005 | Offline status indicator visible | Medium | Unit |
| TC-AVATAR-006 | Correct size variant rendering | Low | Visual |
| TC-AVATAR-007 | No avatar for empty messages | Low | Edge Case |
| **MessageBubble** | | | |
| TC-MSG-001 | User messages right-aligned | High | Layout |
| TC-MSG-002 | Assistant messages left-aligned | High | Layout |
| TC-MSG-003 | Sender role label displayed | High | Unit |
| TC-MSG-004 | Timestamp displayed | Medium | Unit |
| TC-MSG-005 | Markdown rendered correctly | High | Integration |
| TC-MSG-006 | Inline code styled distinctly | High | Visual |
| TC-MSG-007 | Links styled correctly | Medium | Visual |
| TC-MSG-008 | Source toggle visible for assistant | High | Component |
| TC-MSG-009 | Sources expand on click | High | Interaction |
| TC-MSG-010 | Sources collapse on second click | Medium | Interaction |
| TC-MSG-011 | Document name and relevance in source item | High | Component |
| **Typing Indicator** | | | |
| TC-TYPING-001 | Typing indicator visible during streaming | High | Component |
| TC-TYPING-002 | Three animated dots present | Medium | Visual |
| TC-TYPING-003 | Sequential animation delays | Low | Visual |
| TC-TYPING-004 | Indicator hides on completion | High | Integration |
| TC-TYPING-005 | Indicator hides on error | High | Integration |
| **ChatWindow** | | | |
| TC-CHAT-001 | Header with collection selector | High | Component |
| TC-CHAT-002 | Collection selector lists options | High | Component |
| TC-CHAT-003 | New chat button visible | High | Component |
| TC-CHAT-004 | New chat clears messages | High | Integration |
| TC-CHAT-005 | Messages area scrollable | Medium | Layout |
| TC-CHAT-006 | Welcome message when no messages | Medium | Component |
| TC-CHAT-007 | Welcome hidden when messages exist | Medium | Component |
| TC-CHAT-008 | Input textarea rendered | High | Component |
| TC-CHAT-009 | Input placeholder text | Low | Component |
| TC-CHAT-010 | Send button visible | High | Component |
| TC-CHAT-011 | Send disabled when input empty | High | Interaction |
| TC-CHAT-012 | Send enabled with text + collection | High | Interaction |
| TC-CHAT-013 | Enter sends message | High | Interaction |
| TC-CHAT-014 | Shift+Enter inserts newline | Medium | Interaction |
| TC-CHAT-015 | Cancel button during streaming | High | Integration |
| **Animations** | | | |
| TC-ANIM-001 | New messages fade in | Medium | Visual |
| TC-ANIM-002 | Animation duration < 500ms | Low | Visual |
| TC-ANIM-003 | Auto-scroll to bottom | High | Integration |
| **Navigation** | | | |
| TC-NAV-001 | Chat is default landing page | High | Integration |
| TC-NAV-002 | Admin nav removed from main layout | High | Integration |
| TC-NAV-003 | Chat nav exists | Medium | Integration |
| TC-NAV-004 | Admin accessible via /admin | High | Integration |
| TC-NAV-005 | Admin shows API key input | Medium | Integration |
| TC-NAV-006 | Browser back returns to chat | Medium | Integration |
| **Responsive** | | | |
| TC-RESP-001 | Mobile: vertical stacking at 375px | High | Layout |
| TC-RESP-002 | Mobile: input full width | High | Layout |
| TC-RESP-003 | Mobile: no overflow with long text | Medium | Layout |
| TC-RESP-004 | Tablet: no horizontal scroll at 768px | High | Layout |
| TC-RESP-005 | Desktop: max-width constrained messages | Medium | Layout |
| TC-RESP-006 | Mobile: session sidebar collapsed | Medium | Layout |
| TC-RESP-007 | Admin page responsive at 375px | Medium | Layout |
| TC-RESP-008 | Auth card fits mobile viewport | Medium | Layout |
| **Login Page** | | | |
| TC-LOGIN-001 | Login page accessible at /login | High | Component |
| TC-LOGIN-002 | App name and welcome text displayed | Medium | Component |
| TC-LOGIN-003 | Three social provider buttons rendered | High | Component |
| TC-LOGIN-004 | Google button with icon/text | High | Component |
| TC-LOGIN-005 | GitHub button with icon/text | High | Component |
| TC-LOGIN-006 | Discord button with icon/text | High | Component |
| TC-LOGIN-007 | Provider click navigates to KeyCloak | High | Integration |
| TC-LOGIN-008 | Login page responsive on mobile | Medium | Layout |
| TC-LOGIN-009 | Error state for failed auth | Medium | Component |
| TC-LOGIN-010 | Privacy/terms notice displayed | Low | Component |
| **Auth Guard** | | | |
| TC-AUTH-001 | Unauthenticated → redirect to /login | High | Integration |
| TC-AUTH-002 | Authenticated → access chat page | High | Integration |
| TC-AUTH-003 | Logout clears token → redirect to login | High | Integration |
| TC-AUTH-004 | Token persists across page reload | Medium | Integration |
| TC-AUTH-005 | Expired token → show login | High | Integration |
| TC-AUTH-006 | Admin page requires auth | High | Integration |
| TC-AUTH-007 | User avatar displays initials from token | Medium | Integration |

## Running Tests

```bash
# Install dependencies
cd frontend && npm install

# Run all Playwright tests
npx playwright test

# Run specific test file
npx playwright test e2e/message-bubble.spec.ts

# Run with specific project (mobile viewport)
npx playwright test --project=mobile

# Run unit tests
npm run test

# Run both
npm run test && npx playwright test
```

## TDD Workflow

1. Tests are written **before** implementation (this document)
2. Run tests — they will fail (red)
3. Implement the feature as described in each task
4. Run tests again — they should pass (green)
5. Refactor if needed, keeping tests green

## Test Data Strategy

- **Mock data:** Pinia stores should provide mock initial state for components
- **LocalStorage:** Auth tests use `localStorage` manipulation for token simulation
- **Viewport:** Responsive tests use `page.setViewportSize()` for device emulation
- **Selectors:** All components must use `data-testid` attributes for reliable selection
