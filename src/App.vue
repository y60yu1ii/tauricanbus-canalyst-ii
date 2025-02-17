<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface BoardInfo {
  hardware_version: number;
  firmware_version: number;
  serial_number: string;
}

const errorMessage = ref<string | null>(null);
const actionMessage = ref<string | null>(null);

// 監聽 Rust 傳來的錯誤訊息
onMounted(() => {
  listen("error-message", (event) => {
    errorMessage.value = event.payload as string;
  });
});

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
    errorMessage.value = `讀取 Board Info 失敗: ${String(error)}`;
  }
}

// 開啟 CAN 裝置
async function openCanDevice() {
  try {
    const response = await invoke("open_can_device", {
      devType: 4,
      devIndex: 0,
    });
    errorMessage.value = response as string;
    await readBoardInfo();
  } catch (error) {
    errorMessage.value = `開啟 CAN 裝置失敗: ${String(error)}`;
  }
}

// 關閉 CAN 裝置
async function closeCanDevice() {
  try {
    const response = await invoke("stop_can_device", {
      devType: 4,
      devIndex: 0,
    });
    errorMessage.value = response as string;
    boardInfo.value = null;
  } catch (error) {
    errorMessage.value = `關閉 CAN 裝置失敗: ${String(error)}`;
  }
}

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

async function reconnectDevice() {
  if (!selectedBaud.value) {
    errorMessage.value = "請選擇一個波特率。";
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
    actionMessage.value = response as string;
  } catch (error) {
    console.error("重新連線 CAN 裝置錯誤:", error);
    errorMessage.value = "重新連線 CAN 裝置失敗。";
  }
}



const canData = ref<string>("");

onMounted(() => {
  listen("can-data", (event) => {
    canData.value = event.payload as string;
  });
});

async function startReceivingData() {
  try {
    await invoke("start_receiving_data", {
      devType: 4,
      devIndex: 0,
      canChannel: 0,
    });
    actionMessage.value = "開始接收 CAN 資料...";
  } catch (error) {
    errorMessage.value = `開始接收資料失敗: ${String(error)}`;
  }
}

async function stopReceivingData() {
  try {
    const response = await invoke("stop_receiving_data");
      actionMessage.value = response as string;
  } catch (error) {
    errorMessage.value = `停止接收資料失敗: ${String(error)}`;
  }
}

const canMessage = ref<number | null>(null);
async function transmitCanData() {
  if (canMessage.value === null) {
    errorMessage.value = "請輸入要傳送的數據。";
    return;
  }
  try {
    const response = await invoke("transmit_can_data", {
      data: canMessage.value,
      devType: 4,
      devIndex: 0,
      canChannel: 0,
    });
      actionMessage.value = response as string;
  } catch (error) {
    errorMessage.value = `傳送 CAN 數據失敗: ${String(error)}`;
  }
}
</script>

<template>
  <main class="container">
    <h1>CAN 裝置控制</h1>
    <div v-if="errorMessage" class="error-message">
      <p><strong>錯誤:</strong> {{ errorMessage }}</p>
    </div>
    <div v-if="actionMessage" class="error-message">
      <p><strong>動作</strong> {{ actionMessage }}</p>
    </div>


    <section>
      <h2>裝置設定</h2>
      <button @click="openCanDevice">開啟 CAN 裝置</button>
      <button @click="closeCanDevice">關閉 CAN 裝置</button>
    </section>

    <section>
      <h2>裝置資訊</h2>
      <button @click="readBoardInfo">讀取 Board Info</button>
      <div v-if="boardInfo">
        <p><strong>硬體版本：</strong> {{ boardInfo.hardware_version }}</p>
        <p><strong>固件版本：</strong> {{ boardInfo.firmware_version }}</p>
        <p><strong>序列號：</strong> {{ boardInfo.serial_number }}</p>
      </div>
    </section>

    <!-- 設定波特率 -->
    <section>
      <h2>設定波特率</h2>
      <select v-model="selectedBaud">
        <option v-for="baud in baudRates" :key="baud.rate" :value="baud">
          {{ baud.rate }}
        </option>
      </select>
      <button @click="reconnectDevice">設定</button>
    </section>

    <!-- 資料傳送 -->
    <section>
      <h2>傳送 CAN 資料</h2>
      <input v-model.number="canMessage" type="number" placeholder="輸入數據" />
      <button @click="transmitCanData">傳送</button>
    </section>

    <!-- 資料接收 -->
    <section>
      <h2>接收 CAN 資料</h2>
      <button @click="startReceivingData">開始接收</button>
      <button @click="stopReceivingData">停止接收</button>
      <p>{{ canData }}</p>
    </section>
  </main>
</template>
