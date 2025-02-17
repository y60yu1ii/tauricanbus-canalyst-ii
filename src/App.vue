<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 定義裝置資訊結構
interface BoardInfo {
  hardware_version: number;
  firmware_version: number;
  serial_number: string;
}

// 存儲讀取的 Board Info
const boardInfo = ref<BoardInfo | null>(null);

// 讀取 CAN 裝置的 Board Info
async function readBoardInfo() {
  try {
    const response = await invoke<BoardInfo>("read_board_info", {
      devType: 4,
      devIndex: 0,
    });
    boardInfo.value = response;
  } catch (error) {
    console.error("讀取 Board Info 錯誤:", error);
    alert("讀取 Board Info 失敗。");
  }
}

// 開啟 CAN 裝置
async function openCanDevice() {
  try {
    const response = await invoke("open_can_device", {
      devType: 4,
      devIndex: 0,
      can1: 0,
      can2: 1,
    });
    alert(response);
    // ✅ 成功開啟裝置後，自動讀取 Board Info
    await readBoardInfo();
  } catch (error) {
    console.error("開啟 CAN 裝置錯誤:", error);
    alert("開啟 CAN 裝置失敗。");
  }
}

// **關閉 CAN 裝置**
async function closeCanDevice() {
  try {
    const response = await invoke("stop_can_device", {
      devType: 4,
      devIndex: 0,
    });
    alert(response);
    boardInfo.value = null; // ✅ 關閉裝置後清除 Board Info
  } catch (error) {
    console.error("關閉 CAN 裝置錯誤:", error);
    alert("關閉 CAN 裝置失敗。");
  }
}

// 波特率選項
const baudRates = [
  { rate: "10 Kbps", timing0: 0x31, timing1: 0x1C },
  { rate: "20 Kbps", timing0: 0x18, timing1: 0x1C },
  { rate: "40 Kbps", timing0: 0x87, timing1: 0xFF },
  { rate: "50 Kbps", timing0: 0x09, timing1: 0x1C },
  { rate: "80 Kbps", timing0: 0x83, timing1: 0xFF },
  { rate: "100 Kbps", timing0: 0x04, timing1: 0x1C },
  { rate: "125 Kbps", timing0: 0x03, timing1: 0x1C },
  { rate: "200 Kbps", timing0: 0x81, timing1: 0xFA },
  { rate: "250 Kbps", timing0: 0x01, timing1: 0x1C },
  { rate: "400 Kbps", timing0: 0x80, timing1: 0xFA },
  { rate: "500 Kbps", timing0: 0x00, timing1: 0x1C },
  { rate: "666 Kbps", timing0: 0x80, timing1: 0xB6 },
  { rate: "800 Kbps", timing0: 0x00, timing1: 0x16 },
  { rate: "1000 Kbps", timing0: 0x00, timing1: 0x14 },
  { rate: "33.33 Kbps", timing0: 0x09, timing1: 0x6F },
  { rate: "66.66 Kbps", timing0: 0x04, timing1: 0x6F },
  { rate: "83.33 Kbps", timing0: 0x03, timing1: 0x6F },
];

const selectedBaud = ref(baudRates.find((b) => b.rate === "250 Kbps"));

// 重新連線裝置並設置波特率
async function reconnectDevice() {
  if (!selectedBaud.value) {
    alert("請選擇一個波特率。");
    return;
  }
  try {
    const response = await invoke("reconnect_can_device", {
      devType: 4,
      devIndex: 0,
      can1: 0,
      can2: 1,
      timing0: selectedBaud.value.timing0,
      timing1: selectedBaud.value.timing1,
    });
    alert(response);
  } catch (error) {
    console.error("重新連線 CAN 裝置錯誤:", error);
    alert("重新連線 CAN 裝置失敗。");
  }
}

// 開始接收 CAN 資料
async function startReceivingData() {
  try {
    await invoke("start_receiving_data", {
      devType: 4,
      devIndex: 0,
      canChannel: 0,
    });
  } catch (error) {
    console.error("開始接收資料錯誤:", error);
    alert("開始接收資料失敗。");
  }
}

// 停止接收 CAN 資料
async function stopReceivingData() {
  try {
    const response = await invoke("stop_receiving_data");
    alert(response);
  } catch (error) {
    console.error("停止接收資料錯誤:", error);
    alert("停止接收資料失敗。");
  }
}

// 存儲接收到的 CAN 數據
const canData = ref<string>("");

// 監聽 "can-data" 事件
onMounted(() => {
  listen("can-data", (event) => {
    canData.value = event.payload as string;
  });
});
</script>

<template>
  <main class="container">
    <h1>CAN 裝置控制</h1>

    <!-- 裝置設定 -->
    <section>
      <h2>裝置設定</h2>
      <div class="button-group">
        <button @click="openCanDevice">開啟 CAN 裝置</button>
        <button @click="closeCanDevice">關閉 CAN 裝置</button>
      </div>
    </section>

    <!-- 顯示 Board Info -->
    <section v-if="boardInfo">
      <h2>裝置資訊</h2>
      <p><strong>硬體版本：</strong> {{ boardInfo.hardware_version }}</p>
      <p><strong>固件版本：</strong> {{ boardInfo.firmware_version }}</p>
      <p><strong>序列號：</strong> {{ boardInfo.serial_number }}</p>
    </section>

    <!-- 手動讀取 Board Info -->
    <section>
      <h2>讀取裝置資訊</h2>
      <button @click="readBoardInfo">手動讀取 Board Info</button>
    </section>

    <!-- 設定波特率 -->
    <section>
      <h2>設定並重新連線新波特率</h2>
      <label for="baud-select">選擇波特率：</label>
      <select id="baud-select" v-model="selectedBaud">
        <option v-for="baud in baudRates" :key="baud.rate" :value="baud">
          {{ baud.rate }}
        </option>
      </select>
      <button @click="reconnectDevice">設定 & 重新連線</button>
    </section>

    <!-- CAN 資料接收 -->
    <section>
      <h2>CAN 資料接收</h2>
      <button @click="startReceivingData">開始接收資料</button>
      <button @click="stopReceivingData">停止接收資料</button>
      <p>{{ canData }}</p>
    </section>
  </main>
</template>

<style scoped>
.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}
section {
  margin-bottom: 1rem;
}
.button-group {
  display: flex;
  gap: 10px;
  justify-content: center;
}
input,
select,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
  margin: 0.5em;
  cursor: pointer;
}
button:hover {
  border-color: #396cd8;
}
</style>
