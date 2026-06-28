import { writable } from "svelte/store";

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface ChatSession {
  id: number;
  title: string;
  messages: ChatMessage[];
}

export const chats = writable<ChatSession[]>([]);
export const activeChatId = writable<number>(0);
export const isLoading = writable<boolean>(false);