import { decodeToken, extractUserClaims, getTokenExpiry } from "@/api/auth";
import { describe, expect, it } from "vitest";

/**
 * Build a mock JWT string with the given payload.
 * Header: {"alg":"RS256","typ":"JWT"}
 * Signature: mock
 */
function makeMockJwt(payload: Record<string, unknown>): string {
	const header = btoa(JSON.stringify({ alg: "RS256", typ: "JWT" }));
	// Encode payload as UTF-8 then Base64url, mimicking real JWT encoding
	const json = JSON.stringify(payload);
	const bytes = new TextEncoder().encode(json);
	const binary = String.fromCharCode(...bytes);
	const payloadB64 = btoa(binary)
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=+$/, "");
	return `${header}.${payloadB64}.mocksignature`;
}

describe("decodeToken", () => {
	it("decodes a standard JWT payload", () => {
		const token = makeMockJwt({
			sub: "user-123",
			name: "Test User",
			exp: 9999999999,
		});
		const result = decodeToken(token);
		expect(result).not.toBeNull();
		expect(result?.sub).toBe("user-123");
		expect(result?.name).toBe("Test User");
	});

	it("decodes Cyrillic characters correctly (UTF-8 regression)", () => {
		const name = "Николай Марусов";
		const token = makeMockJwt({ sub: "user-456", name });
		const result = decodeToken(token);
		expect(result).not.toBeNull();
		expect(result?.name).toBe(name);
	});

	it("decodes CJK characters correctly", () => {
		const name = "张伟";
		const token = makeMockJwt({ sub: "user-789", name });
		const result = decodeToken(token);
		expect(result).not.toBeNull();
		expect(result?.name).toBe(name);
	});

	it("decodes accented Latin characters correctly", () => {
		const name = "José François Müller";
		const token = makeMockJwt({ sub: "user-101", name });
		const result = decodeToken(token);
		expect(result).not.toBeNull();
		expect(result?.name).toBe(name);
	});

	it("returns null for invalid token (not three parts)", () => {
		expect(decodeToken("not-a-valid-token")).toBeNull();
	});

	it("returns null for token with malformed JSON payload", () => {
		const token = `header.${btoa("not-json")}.sig`;
		expect(decodeToken(token)).toBeNull();
	});

	it("returns null for empty token", () => {
		expect(decodeToken("")).toBeNull();
	});

	it("decodes realm_access.roles for admin check", () => {
		const token = makeMockJwt({
			sub: "admin-1",
			name: "Admin User",
			realm_access: { roles: ["admin", "offline_access"] },
		});
		const result = decodeToken(token);
		expect(result).not.toBeNull();
		const realmAccess = result?.realm_access as { roles?: string[] };
		expect(realmAccess?.roles).toContain("admin");
	});
});

describe("extractUserClaims", () => {
	it("extracts name from name claim", () => {
		const token = makeMockJwt({
			name: "John Doe",
			preferred_username: "johnd",
		});
		expect(extractUserClaims(token).name).toBe("John Doe");
	});

	it("falls back to preferred_username when name is absent", () => {
		const token = makeMockJwt({ preferred_username: "johnd" });
		expect(extractUserClaims(token).name).toBe("johnd");
	});

	it("extracts provider from identity_provider claim", () => {
		const token = makeMockJwt({ name: "Test", provider: "keycloak" });
		expect(extractUserClaims(token).provider).toBe("keycloak");
	});

	it("returns empty for invalid token", () => {
		expect(extractUserClaims("bad.token")).toEqual({});
	});
});

describe("getTokenExpiry", () => {
	it("returns expiry in milliseconds", () => {
		const exp = 1_000_000_000;
		const token = makeMockJwt({ sub: "u1", exp });
		expect(getTokenExpiry(token)).toBe(exp * 1000);
	});

	it("returns 0 when exp is missing", () => {
		const token = makeMockJwt({ sub: "u1" });
		expect(getTokenExpiry(token)).toBe(0);
	});

	it("returns 0 for invalid token", () => {
		expect(getTokenExpiry("bad.token")).toBe(0);
	});
});
