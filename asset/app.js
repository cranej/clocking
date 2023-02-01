const {createApp} = Vue
createApp({
    data() {
        return {
            recentTitles: [],
            detailObject: null ,
            ongoing: new Map(),
            error: null,
            newStart: '',
            report: null,
            queryParam: {'startOffset': "0", "days": "", 'viewType': "daily_detail"},
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
            let unfinished = await (await fetch(url)).json();
            let m = new Map();
            for (const element of unfinished) {
                m.set(element.title, {'item': element, 'notes': ''});
            };
            this.ongoing = m;
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
            await (fetch(url, {method: 'POST', body: this.ongoing.get(title).notes})
                   .then((rep) => {
                       if (rep.ok) {
                           this.getData();
                       } else {
                           this.error = `${rep.status}`;
                       }
                   }).catch((err) => this.error = err))
        },
        async getReport(offset, days, viewType) {
            let offsetParam = isNaN(parseInt(offset, 10)) ? "0" : offset;
            let daysParam = isNaN(parseInt(days, 10)) ? "null" : days;
            let url = `/api/report/${offsetParam}/${daysParam}?view_type=${viewType}`;
            this.report = await (await fetch(url)).text();
        },
        async getItemDetail(title) {
            let url = `/api/latest/${encodeURI(title)}`;
            let obj = await (await fetch(url)).json();
            if (obj != null) {
                obj.id.start = new Date(obj.id.start).toLocaleString();
                obj.end = new Date(obj.end).toLocaleString();
            }
            this.detailObject = obj;
        },
        onQuickReport(offset, days) {
            this.queryParam.startOffset = offset;
            this.queryParam.days = days;

            this.getReport(offset, days, this.queryParam.viewType);
        }
    }
}).mount("#layout");
