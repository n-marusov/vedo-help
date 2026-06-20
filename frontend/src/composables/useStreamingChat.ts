import { getAccessToken } from "@/api/client";
import type { StreamEvent } from "@/api/types";

export function useStreamingChat() {
	async function* streamQuery(
		query: string,
		collectionId: string,
		sessionId?: string,
		signal?: AbortSignal,
	): AsyncGenerator<StreamEvent> {
		const headers: Record<string, string> = {
			"Content-Type": "application/json",
		};
		const token = getAccessToken();
		if (token) {
			headers.Authorization = `Bearer ${token}`;
		}

		const body: Record<string, unknown> = {
			collection_id: collectionId,
			query,
		};
		if (sessionId) {
			body.session_id = sessionId;
		}

		const response = await fetch("/api/query", {
			method: "POST",
			headers,
			body: JSON.stringify(body),
			signal,
		});

		if (!response.ok) {
			yield {
				type: "error",
				text: `Query failed: ${response.statusText}`,
			};
			return;
		}

		const reader = response.body?.getReader();
		if (!reader) {
			yield { type: "error", text: "No response body" };
			return;
		}

		const decoder = new TextDecoder();
		let buffer = "";

		try {
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;

				buffer += decoder.decode(value, { stream: true });
				const lines = buffer.split("\n");
				buffer = lines.pop() || "";

				for (const line of lines) {
					const trimmed = line.trim();
					if (!trimmed) continue;
					try {
						const event: StreamEvent = JSON.parse(trimmed);
						yield event;
					} catch {
						// skip malformed lines
					}
				}
			}

			// Process remaining buffer
			const trimmed = buffer.trim();
			if (trimmed) {
				try {
					const event: StreamEvent = JSON.parse(trimmed);
					yield event;
				} catch {
					// skip malformed
				}
			}
		} catch (err) {
			if (err instanceof Error && err.name === "AbortError") {
				yield { type: "done" };
				return;
			}
			yield {
				type: "error",
				text: err instanceof Error ? err.message : "Stream error",
			};
		}
	}

	return { streamQuery };
}
