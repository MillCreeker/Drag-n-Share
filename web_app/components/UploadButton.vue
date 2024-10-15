<template>
    <div class="flex items-center justify-center">
        <label for="file-upload" class="flex items-center px-4 py-2 bg-gray-200 rounded-lg cursor-pointer">
            <i class="material-icons text-3xl">upload</i>
            <span class="ml-2">Upload File(s)</span>
        </label>
        <input type="file" multiple @change="_uploadFiles" id="file-upload" class="hidden" />
    </div>
</template>

<script setup>
import { uploadFiles, createSession } from '~/public/utils/api';
import { convertFiles } from '~/public/utils/utils';
const route = useRoute();

const { createSessionBeforeUpload, cbRefresh } = defineProps(['createSessionBeforeUpload', 'cbRefresh']);

async function _uploadFiles(event) {
    const files = await convertFiles(event.srcElement.files);

    if (createSessionBeforeUpload) {
        await createSession(files);
        return;
    }

    const sessionId = route.path.split('/')[1];
    await uploadFiles(files, sessionId);

    cbRefresh();
}
</script>