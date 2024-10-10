<template>
    <div class="flex items-center justify-center">
        <label for="file-upload" class="flex items-center px-4 py-2 bg-gray-200 rounded-lg cursor-pointer">
            <i class="material-icons text-3xl">upload</i>
            <span class="ml-2">Upload File(s)</span>
        </label>
        <input type="file" multiple @change="uploadFiles" id="file-upload" class="hidden" />
    </div>
</template>

<script setup>
import { uploadFile, createSession } from '~/public/utils/api';
import { convertFiles } from '~/public/utils/utils';

const { createSessionAfterUpload } = defineProps(['createSessionAfterUpload']);

async function uploadFiles(event) {
    const files = await convertFiles(event.srcElement.files);

    for (let i = 0; i < files.length; i++) {
        await uploadFile(files[i]);
    }

    if (createSessionAfterUpload) {
        await createSession();
    }
}
</script>