<template>
    <div>
        <div class="flex items-center justify-between py-2 px-4 rounded-lg shadow-md"
            :class="isOwner ? 'bg-gray-200' : 'bg-white'">
            <div class="flex items-center">
                <i class="material-icons text-3xl">attach_file</i>
                <p class="ml-2 text-lg">{{ filename }}</p>
            </div>
            <div v-if="isOwner" @click="_deleteFile">
                <i class="material-icons text-3xl text-red-500 cursor-pointer">close</i>
            </div>
            <div v-else @click="_downloadFile">
                <i class="material-icons text-3xl text-orange-500 cursor-pointer">download</i>
            </div>
        </div>
    </div>
</template>

<script setup>
import { onMounted } from 'vue';
import { deleteFile } from '~/public/utils/api';

const route = useRoute();

const { filename, size, isOwner, cbRefresh, cbDownload } = defineProps(['filename', 'size', 'isOwner', 'cbRefresh', 'cbDownload']);

const _deleteFile = async () => {
    const sessionId = route.path.split('/')[1];
    await deleteFile(filename, sessionId);
    cbRefresh();
};

const _downloadFile = async () => {
    await cbDownload(filename);
};
</script>