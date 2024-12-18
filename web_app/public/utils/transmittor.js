import { nextTick } from "vue";
import { generateKeyPair, deriveSharedSecret, convertKeyToBase64, importKeyFromBase64, exportPrivateKeyToBase64, importPrivateKeyFromBase64, exportSharedSecretToBase64, importSharedSecretFromBase64, generateIv, ivToBase64, base64ToIv, arrayBufferToHex, hexToArrayBuffer, encryptData, decryptData, downloadDataUrl, getFile, storeLargeString, getLargeString } from '~/public/utils/utils';

const chunkSize = 32768;

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
    const amountOfChunks = Math.ceil(file.length / chunkSize);

    await trnsAcknwoledgeFileRequest(socket, requestId, base64PublicKey, amountOfChunks, filename);
}

async function trnsHandlePrepareForFileTransfer(socket, requestId, data) {
    console.log('trnsHandlePrepareForFileTransfer');
    const clientPublicKey = await importKeyFromBase64(data.public_key);
    const filename = data.filename;
    const amountOfChunks = data.amount_of_chunks;

    const requestIdCookie = useCookie(requestId);
    requestIdCookie.value = filename;

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
    const chunkNr = data.chunk_nr;
    // console.log('chunkNr', chunkNr);

    if (chunkNr === 0) {
        return;
    }

    const requestIdCookie = useCookie(requestId);

    const file = await getFile(requestIdCookie.value);

    const cutOff = chunkNr * chunkSize;
    const chunk = file.slice((chunkNr - 1) * chunkSize, cutOff);
    const isLastChunk = file.length <= cutOff;

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = await importSharedSecretFromBase64(secretCookie.value);

    const iv = await generateIv();
    const base64Iv = await ivToBase64(iv);

    const encryptedChunk = await encryptData(secret, iv, chunk);
    const hexChunk = arrayBufferToHex(encryptedChunk);

    await trnsAddChunk(socket, requestId, isLastChunk, chunkNr, hexChunk, base64Iv);
}

async function trnsHandleAddChunk(socket, requestId, data) {
    console.log('trnsHandleAddChunk');
    const isLastChunk = data.is_last_chunk;
    const chunkNr = data.chunk_nr;
    const encryptedChunk = hexToArrayBuffer(data.chunk);
    const iv = await base64ToIv(data.iv);

    const secretCookie = useCookie(`${requestId}-secret`);
    const secret = await importSharedSecretFromBase64(secretCookie.value);

    const chunk = await decryptData(secret, iv, encryptedChunk);

    getLargeString(`${requestId}-file`)
        .then(async (file) => {
            file = `${file}${chunk}`;
            await storeLargeString(`${requestId}-file`, file);

            if (isLastChunk) {
                const fileParts = file.split(',');
                const decodedFile = atob(fileParts[1]);

                const requestIdCookie = useCookie(requestId);
                downloadDataUrl(decodedFile, requestIdCookie.value);
            }
        })
        .catch(async (error) => {
            console.error(error);
            await storeLargeString(`${requestId}-file`, chunk);
        });

    await trnsReceivedChunk(socket, requestId, chunkNr);
}



async function trnsAcknwoledgeFileRequest(socket, requestId, publicKey, amountOfChunks, filename) {
    console.log('trnsAcknwoledgeFileRequest');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'acknowledge-file-request',
        data: JSON.stringify({
            request_id: requestId,
            public_key: publicKey,
            amount_of_chunks: amountOfChunks,
            filename: filename,
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

async function trnsAddChunk(socket, requestId, isLastChunk, chunkNr, hexChunk, iv) {
    console.log('trnsAddChunk');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'add-chunk',
        data: JSON.stringify({
            request_id: requestId,
            is_last_chunk: isLastChunk,
            chunk_nr: chunkNr,
            chunk: hexChunk,
            iv: iv
        })
    }));
}

async function trnsReceivedChunk(socket, requestId, chunkNr) {
    console.log('trnsReceivedFile');
    const jwtCookie = useCookie('jwt');

    socket.send(JSON.stringify({
        jwt: jwtCookie.value,
        command: 'received-chunk',
        data: JSON.stringify({
            request_id: requestId,
            chunk_nr: chunkNr
        })
    }));
}