class WaveformProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.currentSample = 0
        this.currentWaveformSample = 0
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
                maxValue: 20000,
                automationRate: "k-rate",
            },
            {
                name: "data0",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data1",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data2",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data3",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data4",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data5",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data6",
                defaultValue: 0,
                automationRate: "k-rate",
            },
            {
                name: "data7",
                defaultValue: 0,
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

    getSample(parameters, index) {
        let parameterOffset = Math.floor(index / 4)
        let valueOffset = Math.floor((index % 4) / 2)
        let nibbleShift = (index % 2 === 0) ? 4 : 0
        let value = Math.floor(parameters[`data${parameterOffset}`][0])
        value >>= (valueOffset * 8);
        value &= 0xFF
        value >>= nibbleShift;
        value &= 0xF
        return 1 - 2 * (value / 15)
    }

    process(inputs, outputs, parameters) {
        const enabled = parameters.trigger[0] > 0.5
        if (enabled) {
            const samplesPerWavelength = Math.floor(sampleRate / parameters.frequency[0])
            const samplesPerWaveformSample = Math.floor(samplesPerWavelength / 32)
            const gain = parameters.gain[0]
            const samples = []
            for (let i = 0; i < 128; i++) {
                if (this.currentSample > samplesPerWaveformSample) {
                    this.currentSample = 0
                    this.currentWaveformSample = (this.currentWaveformSample + 1) % 32
                }
                const value = this.getSample(parameters, this.currentWaveformSample);
                samples.push(value * gain)
                this.currentSample++
            }
            const output = outputs[0];
            output.forEach((channel, channelIndex) => {
                const channelGain = channelIndex === 0 ? parameters.leftChannelGain[0] : parameters.rightChannelGain[0]
                for(let i = 0; i < 128; ++i) {
                    channel[i] = samples[i] * channelGain
                }
            });
        }
        return true;
    }
}

registerProcessor("waveform-processor", WaveformProcessor);