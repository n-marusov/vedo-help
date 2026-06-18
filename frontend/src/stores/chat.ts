import { ApiError, api, getAccessToken } from "@/api/client";
import type {
	Message,
	Session,
	SessionSummary,
	StreamEvent,
} from "@/api/types";
import { defineStore } from "pinia";
import { ref } from "vue";

export const useChatStore = defineStore("chat", () => {
	const messages = ref<Message[]>([]);
	const isLoading = ref(false);
	const activeSessionId = ref<string | null>(null);
	const sessions = ref<SessionSummary[]>([]);
	const error = ref<string | null>(null);

	let abortController: AbortController | null = null;

	function parseNDJSON(
		reader: ReadableStreamDefaultReader<Uint8Array>,
	): ReadableStream<StreamEvent> {
		return new ReadableStream({
			async start(controller) {
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
								controller.enqueue(event);
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
							controller.enqueue(event);
						} catch {
							// skip malformed
						}
					}
				} catch (err) {
					controller.error(err);
				} finally {
					controller.close();
				}
			},
		});
	}

	async function sendMessage(collectionId: string, query: string) {
		isLoading.value = true;
		error.value = null;
		abortController = new AbortController();

		// Add user message optimistically
		const tempUserMsg: Message = {
			id: `temp-${Date.now()}`,
			session_id: activeSessionId.value || "",
			role: "user",
			content: query,
			created_at: new Date().toISOString(),
		};
		messages.value.push(tempUserMsg);

		// Add placeholder assistant message
		const assistantMsg: Message = {
			id: `temp-assist-${Date.now()}`,
			session_id: activeSessionId.value || "",
			role: "assistant",
			content: "",
			created_at: new Date().toISOString(),
		};
		messages.value.push(assistantMsg);

		try {
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
			if (activeSessionId.value) {
				body.session_id = activeSessionId.value;
			}

			const response = await fetch("/api/query", {
				method: "POST",
				headers,
				body: JSON.stringify(body),
				signal: abortController.signal,
			});

			if (!response.ok) {
				throw new ApiError(
					response.status,
					`Query failed: ${response.statusText}`,
				);
			}

			const reader = response.body?.getReader();
			if (!reader) throw new Error("No response body");

			const stream = parseNDJSON(reader);
			const streamReader = stream.getReader();
			let fullContent = "";
			let sources: string | undefined;

			while (true) {
				const { done, value } = await streamReader.read();
				if (done) break;

				switch (value.type) {
					case "chunk": {
						fullContent += value.text || "";
						// Update the last assistant message content
						const lastMsg = messages.value[messages.value.length - 1];
						if (lastMsg?.role === "assistant") {
							lastMsg.content = fullContent;
							// Force reactivity by replacing the array
							messages.value = [...messages.value];
						}
						break;
					}
					case "sources":
						sources = JSON.stringify(value.sources);
						break;
					case "error":
						error.value = value.text || "An error occurred";
						// Remove the placeholder assistant message
						messages.value.pop();
						break;
					case "done": {
						// Finalize the assistant message
						const finalMsg = messages.value[messages.value.length - 1];
						if (finalMsg?.role === "assistant") {
							finalMsg.content = fullContent;
							finalMsg.sources = sources;
							messages.value = [...messages.value];
						}
						break;
					}
				}
			}

			// Refresh sessions after a new query (might have created a session)
			await fetchSessions();
		} catch (err) {
			if (err instanceof Error && err.name === "AbortError") {
				// User cancelled
			} else if (err instanceof ApiError) {
				error.value = err.message;
			} else if (err instanceof Error) {
				error.value = err.message;
			}
			// Remove the placeholder assistant message on error
			if (
				messages.value.length > 0 &&
				messages.value[messages.value.length - 1]?.role === "assistant"
			) {
				messages.value.pop();
			}
		} finally {
			isLoading.value = false;
			abortController = null;
		}
	}

	function cancelStream() {
		if (abortController) {
			abortController.abort();
			abortController = null;
		}
	}

	async function fetchSessions() {
		try {
			const result = await api.get<SessionSummary[]>("/sessions");
			sessions.value = result;
		} catch (err) {
			if (err instanceof ApiError) {
				error.value = err.message;
			}
		}
	}

	async function createSession(collectionId?: string) {
		try {
			const session = await api.post<Session>("/sessions", {
				collection_id: collectionId,
			});
			activeSessionId.value = session.id;
			messages.value = [];
			await fetchSessions();
			return session;
		} catch (err) {
			if (err instanceof ApiError) {
				error.value = err.message;
			}
			return null;
		}
	}

	async function deleteSession(sessionId: string) {
		try {
			await api.del(`/sessions/${sessionId}`);
			if (activeSessionId.value === sessionId) {
				activeSessionId.value = null;
				messages.value = [];
			}
			await fetchSessions();
		} catch (err) {
			if (err instanceof ApiError) {
				error.value = err.message;
			}
		}
	}

	async function loadSession(sessionId: string) {
		try {
			const msgs = await api.get<Message[]>(`/sessions/${sessionId}/messages`);
			messages.value = msgs;
			activeSessionId.value = sessionId;
		} catch (err) {
			if (err instanceof ApiError) {
				error.value = err.message;
			}
		}
	}

	function clearMessages() {
		messages.value = [];
		activeSessionId.value = null;
		error.value = null;
	}

	return {
		messages,
		isLoading,
		activeSessionId,
		sessions,
		error,
		sendMessage,
		cancelStream,
		fetchSessions,
		createSession,
		deleteSession,
		loadSession,
		clearMessages,
	};
});
