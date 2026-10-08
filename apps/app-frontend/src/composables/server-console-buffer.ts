export class ServerConsoleBuffer {
    private readonly chunks: Uint8Array[] = []
    private readonly capacity: number
    private head = 0
    private byteLength = 0

    constructor(capacity: number) {
        this.capacity = capacity
    }

    push(data: Uint8Array) {
        if (data.length >= this.capacity) {
            this.chunks.length = 0
            this.chunks.push(data.slice(-this.capacity))
            this.head = 0
            this.byteLength = this.capacity
            return
        }

        this.chunks.push(data)
        this.byteLength += data.length
        while (this.byteLength > this.capacity && this.head < this.chunks.length - 1) {
            this.byteLength -= this.chunks[this.head++]!.length
        }

        if (this.head >= 1024 && this.head * 2 >= this.chunks.length) {
            this.chunks.splice(0, this.head)
            this.head = 0
        }
    }

    *values(): IterableIterator<Uint8Array> {
        for (let index = this.head; index < this.chunks.length; index++) {
            yield this.chunks[index]!
        }
    }

    get size() {
        return this.byteLength
    }
}
