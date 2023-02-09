class WhiteNoiseProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.lfsr = 0
        this.currentSample = 0
    }

    static get parameterDescriptors() {
        return [
            {
                name: "trigger",
                defaultValue: 0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "gain",
                defaultValue: 1,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "frequency",
                defaultValue: 200,
                minValue: 1,
                maxValue: 44100,
                automationRate: "k-rate",
            },
            {
                name: "width",
                defaultValue: 1,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
        ];
    }

    doTick(short) {
        let bit0 = (this.lfsr & 0x1) === 1
        let bit1 = ((this.lfsr >> 1) & 0x1) === 1
        let newBitValue = !(bit0 !== bit1)
        const mask = (newBitValue << 15) | (short ? (newBitValue << 7) : 0)
        this.lfsr = this.lfsr >> 1
        this.lfsr = (this.lfsr & ~mask) | mask
    }

    process(inputs, outputs, parameters) {
        const enabled = parameters.trigger[0] > 0.5
        if (enabled) {
            const short = parameters.width[0] < 0.5
            const output = outputs[0];
            const samplesPerTick = Math.floor(sampleRate / parameters.frequency[0])
            output.forEach((channel) => {
                for (let i = 0; i < channel.length; i++) {
                    if (this.currentSample > samplesPerTick) {
                        this.currentSample = 0
                        this.doTick(short)
                    }
                    const value = (this.lfsr % 2) === 1 ? -1 : 1;
                    channel[i] = value * parameters.gain[0]
                    this.currentSample++
                }
            });
        }
        return true;
    }
}

registerProcessor("white-noise-processor", WhiteNoiseProcessor);