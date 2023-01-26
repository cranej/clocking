const {createApp} = Vue
createApp({
    data() {
        return {
            recentTitles: [],
            detailObject: null ,
            ongoing: [],
            error: null,
            newStart: '',
        }
    },

    created() {
        this.getData();
    },

    methods: {
        async getRecent() {
            const url = '/api/recent/';
            this.recentTitles = await (await fetch(url)).json()
        },
        async getUnfinished() {
            const url = '/api/unfinished/';
            this.ongoing = await (await fetch(url)).json()
        },

        getData() {
            this.getRecent();
            this.getUnfinished();
            this.error = null;
        },
        async start(title) {
            if (title == null || title.length == 0) {
                this.error = "Empty title";
                return;
            }

            let url = `/api/start/${encodeURI(title)}`;
            await (fetch(url, {method: 'POST'})
                   .then((rep) => {
                       if (rep.ok) {
                           this.getData();
                       } else {
                           this.error = `${rep.status}`;
                       }
                   }).catch((err) => this.error = err))
        },
        async finish(title) {
            let url = `/api/finish/${encodeURI(title)}`;
            await (fetch(url, {method: 'POST', body: ""})
                   .then((rep) => {
                       if (rep.ok) {
                           this.getData();
                       } else {
                           this.error = `${rep.status}`;
                       }
                   }).catch((err) => this.error = err))
        },
        onStart(event) {
            let title = event.target.getAttribute("data-title");
            this.start(title);
        },
        onFinish(event) {
            let title = event.target.getAttribute("data-title");
            this.finish(title);
        },
    }
}).mount("#layout");
