/// Delete keys with empty values from requests
document.body.addEventListener('htmx:configRequest', function(evt) {
    const params = evt.detail.parameters;

    for (const key in params) {
        if (params.hasOwnProperty(key) && params[key].trim() === '') {
            delete params[key];
        }
    }

    // If a corresponding xyz-rel is set, skip the xyz-fixed
    if (evt.detail.elt.id == "metric-chart-form") {
      if (params.hasOwnProperty("start-rel")) {
        delete params["start-fixed"];
      }

      if (params.hasOwnProperty("end-rel")) {
        delete params["end-fixed"];
      }
    }
});
