<script lang="ts">
  import { chats, activeChatId, type ChatSession } from "./store";
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";

  let editingId: number | null = null;
  let editTitle: string = "";
  let editInputRef: HTMLInputElement;

  async function createNewChat() {
    const newChat: ChatSession = {
      id: Date.now(),
      title: "Unknown_Connection",
      messages: []
    };
    
    chats.update(current => [newChat, ...current]);
    activeChatId.set(newChat.id);
    
    await invoke("save_chats", { chatsJson: JSON.stringify($chats) });
  }

  function selectChat(id: number) {
    if (editingId === null) {
      activeChatId.set(id);
    }
  }

  async function startEditing(chat: ChatSession, event: Event) {
    event.stopPropagation(); 
    editingId = chat.id;
    editTitle = chat.title;
    
    await tick();
    if (editInputRef) {
      editInputRef.focus();
      editInputRef.select(); 
    }
  }

  async function saveEdit() {
    if (editingId !== null) {
      const finalTitle = editTitle.trim() === "" ? "Unnamed_Signal" : editTitle.trim();
      
      chats.update(currentChats => {
        const chatIndex = currentChats.findIndex(c => c.id === editingId);
        if (chatIndex !== -1) {
          currentChats[chatIndex].title = finalTitle;
        }
        return currentChats;
      });

      editingId = null;
      await invoke("save_chats", { chatsJson: JSON.stringify($chats) });
    }
  }

  function handleEditKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      saveEdit();
    } else if (event.key === "Escape") {
      editingId = null; 
    }
  }

  async function deleteChat(id: number, event: Event) {
    event.stopPropagation();
    
    let currentChatsList: ChatSession[] = [];
    chats.update(currentChats => {
      currentChatsList = currentChats.filter(c => c.id !== id);
      return currentChatsList;
    });

    if (currentChatsList.length === 0) {
      const fallbackChat: ChatSession = { id: Date.now(), title: "Unknown_Connection", messages: [] };
      chats.set([fallbackChat]);
      activeChatId.set(fallbackChat.id);
      await invoke("save_chats", { chatsJson: JSON.stringify([fallbackChat]) });
    } else {
      if ($activeChatId === id) {
        activeChatId.set(currentChatsList[0].id);
      }
      await invoke("save_chats", { chatsJson: JSON.stringify(currentChatsList) });
    }
  }
</script>

<aside class="w-72 bg-[#050505] text-gray-400 flex flex-col shadow-2xl z-20 transition-all border-r border-[#1a1a1a] relative overflow-hidden">
  
  <div class="absolute inset-0 opacity-10 pointer-events-none" 
       style="background-image: repeating-linear-gradient(0deg, transparent, transparent 2px, #fff 2px, #fff 4px); background-size: 100% 4px;">
  </div>

  <div class="p-5 border-b border-[#1a1a1a] relative z-10">
    <button
      on:click={createNewChat}
      class="w-full flex items-center justify-center gap-3 bg-[#0d1117] hover:bg-[#151b23] border border-[#30363d] hover:border-cyan-500/50 text-gray-300 hover:text-cyan-400 font-mono text-sm py-3 rounded-lg transition-all group shadow-[0_0_15px_rgba(0,0,0,0.5)]"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 group-hover:animate-pulse" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="square" stroke-linejoin="miter" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
      </svg>
      [Connect_to_Signal]
    </button>
  </div>

  <div class="flex-1 overflow-y-auto p-3 space-y-1 relative z-10 lain-scroll">
    {#each $chats as chat}
      <div class="relative group">
        <button
          on:click={() => selectChat(chat.id)}
          class="w-full text-left px-4 py-3 rounded-md text-sm transition-all border {
            $activeChatId === chat.id
              ? 'bg-[#0d1117] text-gray-100 font-medium border-[#30363d] shadow-[inset_2px_0_0_0_#06b6d4]'
              : 'border-transparent text-gray-500 hover:bg-[#0d1117]/50 hover:text-gray-300'
          }"
        >
          {#if editingId === chat.id}
            <input
              bind:this={editInputRef}
              bind:value={editTitle}
              on:keydown={handleEditKeydown}
              on:blur={saveEdit}
              class="w-full bg-[#050505] text-cyan-400 border border-cyan-500/50 outline-none px-2 py-1 rounded font-mono text-xs focus:shadow-[0_0_8px_rgba(6,182,212,0.3)]"
              spellcheck="false"
            />
          {:else}
            <div class="truncate font-mono pr-12 text-xs tracking-wider">
              <span class={$activeChatId === chat.id ? 'text-cyan-500' : 'text-gray-600'}>&gt; </span> 
              {chat.title}
            </div>
          {/if}
        </button>

        {#if editingId !== chat.id}
          <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              on:click={(e) => startEditing(chat, e)}
              class="p-1.5 text-gray-500 hover:text-cyan-400 transition-colors"
              title="Rename"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor">
                <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z" />
              </svg>
            </button>
            <button
              on:click={(e) => deleteChat(chat.id, e)}
              class="p-1.5 text-gray-500 hover:text-red-500 transition-colors"
              title="Delete Connection"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd" />
              </svg>
            </button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .lain-scroll::-webkit-scrollbar {
    width: 6px;
  }
  .lain-scroll::-webkit-scrollbar-track {
    background: transparent;
  }
  .lain-scroll::-webkit-scrollbar-thumb {
    background: #1f2937;
    border-radius: 10px;
  }
  .lain-scroll::-webkit-scrollbar-thumb:hover {
    background: #06b6d4; /* Cyber Cyan */
    box-shadow: 0 0 10px #06b6d4;
  }
</style>