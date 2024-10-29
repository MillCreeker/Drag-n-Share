<template>
    <div>
        <div class="flex items-center justify-between py-2 px-4 rounded-lg shadow-md"
            :class="isOwner ? 'bg-gray-200' : 'bg-white'">
            <div class="flex items-center">
                <i class="material-icons text-3xl">attach_file</i>
                <p class="ml-2 text-lg">{{ filename }} ({{ convSize }})</p>
            </div>
            <div v-if="isOwner" @click="_deleteFile">
                <i class="material-icons text-3xl text-red-500 cursor-pointer">close</i>
            </div>
            <div v-else>
                <div class="relative w-full bg-orange-200 rounded-lg h-2" >
                    <div class="absolute top-0 left-0 bg-orange-500 rounded-lg h-2"
                        :style="{ width: `${(progress / total) * 100}%` }"></div>
                </div>
                <i class="material-icons text-3xl text-orange-500 cursor-pointer" @click="_downloadFile">download</i>
            </div>
        </div>
    </div>
</template>

<script setup>
import { onMounted } from 'vue';
import { deleteFile } from '~/public/utils/api';

const route = useRoute();

const { filename, size, isOwner, cbRefresh, cbDownload } = defineProps(['filename', 'size', 'isOwner', 'cbRefresh', 'cbDownload']);

let convSize = ref('');

let total = ref(100);
let progress = ref(50);

const updateProgress = (newProgress, newTotal) => {
    progress.value = newProgress;
    if (newTotal === null) {
        return;
    }
    total.value = newTotal;
}

defineExpose({ updateProgress });

const _deleteFile = async () => {
    const sessionId = route.path.split('/')[1];
    await deleteFile(filename, sessionId);
    cbRefresh();
};

const _downloadFile = async () => {
    await cbDownload(filename);
};

function convertSize(size) {
    const units = ['b', 'kb', 'mb', 'gb', 'tb'];
    let index = 0;
    let newSize = size;

    while (newSize >= 1024 && index < units.length - 1) {
        newSize /= 1024;
        index++;
    }

    return `${newSize.toFixed(1)} ${units[index]}`;
}

onMounted(() => {
    convSize.value = convertSize(size);
});
</script>
