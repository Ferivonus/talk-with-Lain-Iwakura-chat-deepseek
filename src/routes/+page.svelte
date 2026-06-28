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
        const initialChat = { id: Date.now(), title: "Signal Entry", messages: [] };
        chats.set([initialChat]);
        await saveChatsToFile([initialChat]);
      }
      activeChatId.set($chats[0].id);
    } catch (err) {
      console.error("Loading error:", err);
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
      console.error("Save error:", err);
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

<main class="h-screen w-screen bg-[#050505] flex overflow-hidden font-sans text-gray-300">
  <Sidebar />

  <section class="flex-1 flex flex-col h-full relative bg-[#0a0a0a]">
    
    <header class="bg-[#0a0a0a]/80 backdrop-blur-md p-5 flex items-center justify-between shadow-md border-b border-[#1a1a1a] z-10">
      <div class="flex items-center gap-4">
        <div class="w-11 h-11 rounded-full bg-[#0d1117] border border-[#30363d] flex items-center justify-center text-cyan-500 font-bold text-xl shadow-[0_0_10px_rgba(6,182,212,0.2)]">
          L
        </div>
        <div>
          <h1 class="text-xl font-bold text-gray-100 tracking-wide">Lain AI</h1>
          <p class="text-xs text-cyan-600/80 font-mono tracking-widest mt-0.5">SIGNAL PROTOCOL v1.0</p>
        </div>
      </div>
      
      {#if $isLoading}
        <div class="flex items-center gap-2 px-3 py-1.5 bg-[#0d1117] border border-[#30363d] rounded-full shadow-inner">
          <span class="relative flex h-2.5 w-2.5">
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-cyan-500"></span>
          </span>
          <span class="text-[10px] font-mono text-cyan-500 uppercase tracking-widest">Transmitting</span>
        </div>
      {/if}
    </header>

    <div bind:this={chatContainer} class="flex-1 overflow-y-auto p-4 md:p-8 space-y-8 scroll-smooth lain-scroll">
      {#if !activeChat || !activeChat.messages || activeChat.messages.length === 0}
        <div class="h-full flex flex-col items-center justify-center text-gray-600 opacity-80">
          <svg class="w-24 h-24 mb-6 text-[#1a1a1a]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
          </svg>
          <p class="text-xl font-mono tracking-tight text-gray-500">"The Signal is vast and infinite..."</p>
        </div>
      {/if}

      {#if activeChat && activeChat.messages}
        {#each activeChat.messages as msg}
          {#if msg.content !== "" || (msg.content === "" && $isLoading)}
            <div class="flex {msg.role === 'user' ? 'justify-end' : 'justify-start'} group">
              
              {#if msg.role === 'assistant'}
                <div class="w-8 h-8 rounded-full bg-[#0d1117] border border-[#30363d] text-cyan-500 flex items-center justify-center text-xs font-bold mr-3 mt-1 shadow-sm shrink-0">
                  L
                </div>
              {/if}

              <div class="max-w-[85%] md:max-w-[80%] rounded-2xl p-5 shadow-lg transition-all {
                msg.role === 'user'
                  ? 'bg-cyan-700/80 text-white rounded-br-none shadow-cyan-900/20'
                  : 'bg-[#151b23] border border-[#30363d] text-gray-300 rounded-bl-none shadow-black/50'
              }">
                {#if msg.role === 'user'}
                  <div class="whitespace-pre-wrap leading-relaxed">{msg.content}</div>
                {:else}
                  {#if msg.content === "" && $isLoading}
                    <div class="flex space-x-1 h-5 items-center opacity-50">
                       <div class="w-2 h-4 bg-cyan-500 animate-pulse"></div>
                    </div>
                  {:else}
                    <div class="prose prose-sm md:prose-base max-w-none prose-pre:bg-[#050505] prose-pre:p-0 prose-pre:rounded-xl prose-pre:border prose-pre:border-[#30363d] prose-invert prose-cyan leading-relaxed">
                      {@html marked.parse(msg.content)}
                      
                      {#if $isLoading && msg === activeChat.messages[activeChat.messages.length - 1]}
                        <span class="inline-block w-2 h-4 ml-1 bg-cyan-500 animate-pulse align-middle shadow-[0_0_8px_#06b6d4]"></span>
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
          <div class="bg-red-900/20 text-red-400 px-4 py-3 rounded-xl border border-red-900/50 text-sm font-mono shadow-sm flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
            </svg>
            <span>{errorMessage}</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="p-5 bg-[#0a0a0a] border-t border-[#1a1a1a] shadow-[0_-10px_30px_rgba(0,0,0,0.5)] z-10">
      <div class="max-w-5xl mx-auto relative flex items-end gap-3 bg-[#0d1117] p-2 rounded-2xl border border-[#30363d] focus-within:ring-1 focus-within:ring-cyan-500/50 focus-within:border-cyan-500 transition-all">
        <textarea
          bind:this={textAreaRef}
          bind:value={userInput}
          on:keydown={handleKeydown}
          on:input={autoResize}
          rows="1"
          class="w-full max-h-40 py-3 px-4 bg-transparent border-transparent outline-none resize-none overflow-y-auto text-gray-200 placeholder-gray-600 font-mono text-sm lain-scroll"
          placeholder="Transmit to the Signal..."
          disabled={$isLoading}
        ></textarea>

        <button
          on:click={handleSend}
          disabled={$isLoading || !userInput.trim()}
          class="bg-cyan-700 hover:bg-cyan-600 disabled:bg-[#1a1a1a] disabled:text-gray-600 text-white p-3 rounded-xl transition-all flex items-center justify-center w-12 h-12 shadow-md mb-0.5 mr-0.5 shrink-0"
          title="Transmit"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 ml-0.5" viewBox="0 0 20 20" fill="currentColor">
            <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z" />
          </svg>
        </button>
      </div>
    </div>
  </section>
</main>