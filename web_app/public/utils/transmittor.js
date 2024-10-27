import { nextTick } from "vue";
import { generateKeyPair, deriveSharedSecret, convertKeyToBase64, importKeyFromBase64, exportPrivateKeyToBase64, importPrivateKeyFromBase64, exportSharedSecretToBase64, importSharedSecretFromBase64, ivToBase64, base64ToIv, encryptData, decryptData, downloadDataUrl, getFile } from '~/public/utils/utils';

export function trnsRegister(socket) {
    console.log('trnsRegister');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'register',
        data: JSON.stringify({})
    }));
}

export async function trnsRequestFile(socket, filename) {
    console.log('trnsRequestFile');
    const jwtCookie = useCookie('jwt');

    const keyPair = await generateKeyPair();

    const publicKeyCookie = useCookie(`${filename}-publicKey`);
    const privateKeyCookie = useCookie(`${filename}-privateKey`);

    const base64PublicKey = await convertKeyToBase64(keyPair.publicKey);
    const base64PrivateKey = await exportPrivateKeyToBase64(keyPair.privateKey);

    publicKeyCookie.value = base64PublicKey;
    privateKeyCookie.value = base64PrivateKey;

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'request-file',
        data: JSON.stringify({
            public_key: base64PublicKey,
            filename: filename
        })
    }));
}

export async function trnsWsHandleMessage(socket, message) {
    console.log('trnsWsHandleMessage');
    console.log('Received:', message);
    const requestId = message.request_id;
    const data = message.data;

    switch (message.command) {
        case 'acknowledge-file-request':
            await trnsHandleAcknowledgeFileRequest(socket, requestId, data);
            break;
        case 'prepare-for-file-transfer':
            await trnsHandlePrepareForFileTransfer(socket, requestId, data);
            break;
        case 'send-next-chunk':
            await trnsHandleSendNextChunk(socket, requestId, data);
            break;
        case 'add-chunk':
            await trnsHandleAddChunk(socket, requestId, data);
            break;
        default:
            console.error('Unknown command:', message.command);
            break;
    }
}

async function trnsHandleAcknowledgeFileRequest(socket, requestId, data) {
    console.log('trnsHandleAcknowledgeFileRequest');
    const clientPublicKey = await importKeyFromBase64(data.public_key);
    const filename = data.filename;
    const userId = data.user_id;

    const requestIdCookie = useCookie(requestId);
    requestIdCookie.value = filename;

    const keyPair = await generateKeyPair();

    const secretObj = await deriveSharedSecret(keyPair.privateKey, clientPublicKey);

    const publicKeyCookie = useCookie(`${requestId}-publicKey`);
    const privateKeyCookie = useCookie(`${requestId}-privateKey`);
    const secretCookie = useCookie(`${requestId}-secret`);

    const base64PublicKey = await convertKeyToBase64(keyPair.publicKey);
    const base64PrivateKey = await exportPrivateKeyToBase64(keyPair.privateKey);

    publicKeyCookie.value = base64PublicKey;
    privateKeyCookie.value = base64PrivateKey;
    secretCookie.value = await exportSharedSecretToBase64(secretObj);

    await nextTick(); // ensure cookie is assigned

    const file = await getFile(filename);
    const amountOfChunks = Math.ceil(file.length / 1024)

    await trnsAcknwoledgeFileRequest(socket, base64PublicKey, amountOfChunks, filename, userId);
}

async function trnsHandlePrepareForFileTransfer(socket, requestId, data) {
    console.log('trnsHandlePrepareForFileTransfer');
    const clientPublicKey = await importKeyFromBase64(data.public_key);
    const filename = data.filename;
    const amountOfChunks = data.amount_of_chunks;

    const publicKeyCookie = useCookie(`${filename}-publicKey`);
    const privateKeyCookie = useCookie(`${filename}-privateKey`);

    const newPublicKeyCookie = useCookie(`${requestId}-publicKey`);
    const newPrivateKeyCookie = useCookie(`${requestId}-privateKey`);

    const privateKey = await importPrivateKeyFromBase64(privateKeyCookie.value);

    newPublicKeyCookie.value = publicKeyCookie.value;
    newPrivateKeyCookie.value = privateKeyCookie.value;

    publicKeyCookie.value = null;
    privateKeyCookie.value = null;

    const secretObj = await deriveSharedSecret(privateKey, clientPublicKey);
    const secret = await exportSharedSecretToBase64(secretObj);
    const secretCookie = useCookie(`${requestId}-secret`);
    secretCookie.value = secret;

    const chunkAmountCookie = useCookie(`${requestId}-chunkAmount`);
    chunkAmountCookie.value = amountOfChunks;

    await trnsReadyForFileTransfer(socket, requestId);
}

async function trnsHandleSendNextChunk(socket, requestId, data) {
    console.log('trnsHandleSendNextChunk');
    const lastChunkNr = data.last_chunk_nr;

    const requestIdCookie = useCookie(requestId);

    console.log(requestIdCookie.value);
    const file = await getFile(requestIdCookie.value);

    const cutOff = lastChunkNr * 1024 + 1024;
    const chunk = file.slice(lastChunkNr * 1024, cutOff);
    const isLastChunk = file.length <= cutOff;

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = await importSharedSecretFromBase64(secretCookie.value);

    const iv = await generateIv();
    const base64Iv = await ivToBase64(iv);

    const encryptedChunk = await encryptData(secret, iv, chunk);

    await trnsAddChunk(socket, requestId, isLastChunk, lastChunkNr + 1, encryptedChunk, base64Iv);

    console.log(filename, lastChunkNr + 1);
}

async function trnsHandleAddChunk(socket, requestId, data) {
    console.log('trnsHandleAddChunk');
    const isLastChunk = data.is_last_chunk;
    const chunkNr = data.chunk_nr;
    const encryptedChunk = data.chunk;
    const iv = await base64ToIv(data.iv);

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = await importSharedSecretFromBase64(secretCookie.value);

    const chunk = await decryptData(secret, iv, encryptedChunk);

    const fileCookie = useCookie(`${requestId}-file`);
    let file = fileCookie.value;
    file = [file.slice(0, chunkNr * 1024), chunk, file.slice(chunkNr * 1024)].join('')
    fileCookie.value = file;

    if (isLastChunk) {
        console.log(file);
        downloadDataUrl(fileCookie.value, 'test.sh');
    }

    console.log(requestId, chunkNr);
}



async function trnsAcknwoledgeFileRequest(socket, publicKey, amountOfChunks, filename, userId) {
    console.log('trnsAcknwoledgeFileRequest');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'acknowledge-file-request',
        data: JSON.stringify({
            public_key: publicKey,
            amount_of_chunks: amountOfChunks,
            filename: filename,
            user_id: userId
        })
    }));
}

async function trnsReadyForFileTransfer(socket, requestId) {
    console.log('trnsReadyForFileTransfer');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'ready-for-file-transfer',
        data: JSON.stringify({
            request_id: requestId
        })
    }));
}

async function trnsAddChunk(socket, requestId, isLastChunk, chunkNr, encryptedChunk, iv) {
    console.log('trnsAddChunk');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'add-chunk',
        data: JSON.stringify({
            request_id: requestId,
            is_last_chunk: isLastChunk,
            chunk_nr: chunkNr,
            chunk: encryptedChunk,
            iv: iv
        })
    }));
}