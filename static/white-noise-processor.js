class WhiteNoiseProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.lfsr = [...Array(15).fill(false)]
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
                defaultValue: 0,
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
                defaultValue: 0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "leftChannelGain",
                defaultValue: 1.0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
            {
                name: "rightChannelGain",
                defaultValue: 1.0,
                minValue: 0,
                maxValue: 1,
                automationRate: "k-rate",
            },
        ];
    }

    doTick(short) {
        const newBitValue = this.lfsr[0] === this.lfsr[1]
        this.lfsr.shift()
        this.lfsr.push(newBitValue)
        if (short) {
            this.lfsr[6] = newBitValue
        }
    }

    process(inputs, outputs, parameters) {
        const enabled = parameters.trigger[0] > 0.5
        if (enabled) {
            const gain = parameters.gain[0]
            const short = parameters.width[0] > 0.5
            const output = outputs[0];
            const samplesPerTick = Math.floor(sampleRate / parameters.frequency[0])
            const samples = []
            for (let i = 0; i < 128; i++) {
                if (this.currentSample > samplesPerTick) {
                    this.currentSample = 0
                    this.doTick(short)
                }
                const value = this.lfsr[0] ? -1 : 1;
                samples.push(value * gain)
                this.currentSample++
            }
            output.forEach((channel, channelIndex) => {
                const channelGain = channelIndex === 0 ? parameters.leftChannelGain[0] : parameters.rightChannelGain[0]
                for(let i = 0; i < 128; ++i) {
                    channel[i] = samples[i] * channelGain
                }
            });
        } else {
            this.lfsr = [...Array(15).fill(false)]
        }
        return true;
    }
}

registerProcessor("white-noise-processor", WhiteNoiseProcessor);