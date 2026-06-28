<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick, onDestroy } from "svelte";

  import { Marked } from "marked";
  import { markedHighlight } from "marked-highlight";
  import hljs from "highlight.js";
  import Sidebar from "../lib/Sidebar.svelte";
  
  import type { ChatSession } from "../lib/store";
  import { chats, activeChatId, isLoading } from "../lib/store";

  const marked = new Marked(
    markedHighlight({
      langPrefix: 'hljs language-',
      highlight(code, lang) {
        const language = hljs.getLanguage(lang) ? lang : 'plaintext';
        return hljs.highlight(code, { language }).value;
      }
    })
  );

  let userInput: string = "";
  let errorMessage: string = "";
  let chatContainer: HTMLElement;
  
  let textAreaRef: HTMLTextAreaElement;

  let unlistenChunk: () => void;
  let unlistenDone: () => void;

  $: activeChat = $chats.find(c => c.id === $activeChatId) || { id: 0, title: "", messages: [] };

  $: if (!$isLoading && textAreaRef) {
    tick().then(() => {
      textAreaRef.focus();
    });
  }

  function autoResize() {
    if (textAreaRef) {
      textAreaRef.style.height = "auto"; 
      textAreaRef.style.height = textAreaRef.scrollHeight + "px"; 
    }
  }

  onMount(async () => {
    try {
      const savedChatsJson = await invoke<string>("load_chats");
      const parsedChats: ChatSession[] = JSON.parse(savedChatsJson);

      if (parsedChats && parsedChats.length > 0) {
        chats.set(parsedChats);
      } else {
        const initialChat = { id: Date.now(), title: "Wired Girişi", messages: [] };
        chats.set([initialChat]);
        await saveChatsToFile([initialChat]);
      }
      activeChatId.set($chats[0].id);
    } catch (err) {
      console.error("Yükleme hatası:", err);
    }

    unlistenChunk = await listen<string>("chat-stream-chunk", (event) => {
      chats.update(currentChats => {
        const chatIndex = currentChats.findIndex(c => c.id === $activeChatId);
        if (chatIndex === -1) return currentChats;

        const chat = currentChats[chatIndex];
        const lastMessage = chat.messages[chat.messages.length - 1];

        if (lastMessage && lastMessage.role === "assistant") {
          lastMessage.content += event.payload;
        }
        
        return currentChats;
      });
      scrollToBottom();
    });

    unlistenDone = await listen("chat-stream-done", async () => {
      isLoading.set(false);
      await saveChatsToFile($chats);
    });
  });

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
  });

  async function saveChatsToFile(currentChats: ChatSession[]) {
    try {
      await invoke("save_chats", { chatsJson: JSON.stringify(currentChats) });
    } catch (err) {
      console.error("Kaydetme hatası:", err);
    }
  }

  const scrollToBottom = async () => {
    await tick(); 
    if (chatContainer) {
      chatContainer.scrollTop = chatContainer.scrollHeight;
    }
  };

  async function handleSend() {
    if (!userInput.trim() || $isLoading) return;

    const currentInput = userInput;
    userInput = "";
    errorMessage = "";
    
    if (textAreaRef) {
      textAreaRef.style.height = "auto";
    }

    isLoading.set(true);

    let updatedMessages: any[] = [];
    
    chats.update(currentChats => {
      const chatIndex = currentChats.findIndex(c => c.id === $activeChatId);
      if (chatIndex === -1) return currentChats;

      let chat = currentChats[chatIndex];
      
      if (chat.messages.length === 0) {
        chat.title = currentInput.length > 20 ? currentInput.substring(0, 20) + "..." : currentInput;
      }

      chat.messages.push({ role: "user", content: currentInput });
      chat.messages.push({ role: "assistant", content: "" });
      
      updatedMessages = [...chat.messages]; 
      return currentChats;
    });

    await saveChatsToFile($chats);
    scrollToBottom();

    try {
      const apiMessages = updatedMessages.filter(m => m.content !== "");
      await invoke("call_deepseek", { messages: apiMessages });
    } catch (error) {
      errorMessage = error as string;
      isLoading.set(false);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  }
</script>

<main class="h-screen w-screen bg-[#f3f4f6] flex overflow-hidden font-sans text-gray-800">
  <Sidebar />

  <section class="flex-1 flex flex-col h-full relative bg-white">
    <header class="bg-white/80 backdrop-blur-md p-5 flex items-center justify-between shadow-sm border-b border-gray-100 z-10">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-full bg-slate-900 flex items-center justify-center text-white font-bold text-lg shadow-inner">
          L
        </div>
        <div>
          <h1 class="text-xl font-bold text-gray-900 leading-tight">Lain AI</h1>
          <p class="text-xs text-gray-500 font-medium tracking-wide">Wired Protocol v1.0</p>
        </div>
      </div>
      
      {#if $isLoading}
        <div class="flex items-center gap-2 px-3 py-1.5 bg-gray-100 rounded-full">
          <span class="relative flex h-2.5 w-2.5">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-blue-500"></span>
          </span>
          <span class="text-xs font-semibold text-gray-600 uppercase tracking-wider">Ağa Bağlanıyor</span>
        </div>
      {/if}
    </header>

    <div bind:this={chatContainer} class="flex-1 overflow-y-auto p-4 md:p-8 space-y-8 scroll-smooth">
      {#if !activeChat || !activeChat.messages || activeChat.messages.length === 0}
        <div class="h-full flex flex-col items-center justify-center text-gray-400 opacity-70">
          <svg class="w-24 h-24 mb-6 text-gray-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
          </svg>
          <p class="text-xl font-medium tracking-tight">"Ağ uçsuz bucaksız..."</p>
        </div>
      {/if}

      {#if activeChat && activeChat.messages}
        {#each activeChat.messages as msg}
          {#if msg.content !== "" || (msg.content === "" && $isLoading)}
            <div class="flex {msg.role === 'user' ? 'justify-end' : 'justify-start'} group">
              
              {#if msg.role === 'assistant'}
                <div class="w-8 h-8 rounded-full bg-slate-800 text-white flex items-center justify-center text-xs font-bold mr-3 mt-1 shadow-sm shrink-0">
                  L
                </div>
              {/if}

              <div class="max-w-[85%] md:max-w-[80%] rounded-2xl p-5 shadow-sm transition-all {
                msg.role === 'user'
                  ? 'bg-blue-600 text-white rounded-br-none shadow-blue-500/20'
                  : 'bg-white border border-gray-100 text-gray-800 rounded-bl-none shadow-gray-200/50'
              }">
                {#if msg.role === 'user'}
                  <div class="whitespace-pre-wrap leading-relaxed">{msg.content}</div>
                {:else}
                  {#if msg.content === "" && $isLoading}
                    <div class="flex space-x-1 h-5 items-center opacity-50">
                       <div class="w-2 h-4 bg-gray-400 animate-pulse"></div>
                    </div>
                  {:else}
                    <div class="prose prose-sm md:prose-base max-w-none prose-pre:bg-[#0d1117] prose-pre:p-0 prose-pre:rounded-xl prose-pre:shadow-lg prose-blue leading-relaxed">
                      {@html marked.parse(msg.content)}
                      
                      {#if $isLoading && msg === activeChat.messages[activeChat.messages.length - 1]}
                        <span class="inline-block w-2 h-4 ml-1 bg-gray-400 animate-pulse align-middle"></span>
                      {/if}
                    </div>
                  {/if}
                {/if}
              </div>
            </div>
          {/if}
        {/each}
      {/if}

      {#if errorMessage}
        <div class="flex justify-center mt-4">
          <div class="bg-red-50 text-red-600 px-4 py-3 rounded-xl border border-red-200 text-sm font-medium shadow-sm flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
            </svg>
            <span>{errorMessage}</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="p-5 bg-white border-t border-gray-100 shadow-[0_-4px_20px_-15px_rgba(0,0,0,0.1)] z-10">
      <div class="max-w-5xl mx-auto relative flex items-end gap-3 bg-gray-50 p-2 rounded-2xl border border-gray-200 focus-within:ring-2 focus-within:ring-blue-500/20 focus-within:border-blue-400 transition-all">
        <textarea
          bind:this={textAreaRef}
          bind:value={userInput}
          on:keydown={handleKeydown}
          on:input={autoResize}
          rows="1"
          class="w-full max-h-40 py-3 px-4 bg-transparent border-transparent outline-none resize-none overflow-y-auto text-gray-800 placeholder-gray-400"
          placeholder="Wired'a bir mesaj gönder..."
          disabled={$isLoading}
        ></textarea>

        <button
          on:click={handleSend}
          disabled={$isLoading || !userInput.trim()}
          class="bg-blue-600 hover:bg-blue-500 disabled:bg-gray-300 disabled:text-gray-500 text-white p-3 rounded-xl transition-all flex items-center justify-center w-12 h-12 shadow-sm mb-0.5 mr-0.5 shrink-0"
          title="Gönder"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 ml-1" viewBox="0 0 20 20" fill="currentColor">
            <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z" />
          </svg>
        </button>
      </div>
    </div>
  </section>
</main>