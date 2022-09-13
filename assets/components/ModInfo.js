export class ModInfo extends Element {
  image = null;

  this(props) {
    this.props = props;
    this.image = Window.this.api.preview(this.props.mod.hash);
  }

  render() {
    const mod = this.props.mod?.meta;
    return mod ? (
      <div styleset={__DIR__ + "ModInfo.css#ModInfo"}>
        {this.image ? <img class="preview" src={this.image} /> : []}
        <Row key="Name" val={mod.name} />
        <Row key="Version" val={mod.version.toPrecision(1)} />
        <Row key="Category" val={mod.category} />
        <Row key="Author" val={mod.author} />
        {mod.url ? <Row key="Webpage" val={mod.url} /> : []}
        <Long key="Description" val={mod.description} />
        {mod.option_groups?.length > 0 ? (
          <Long
            key="Options"
            val={
              <div class="hbox">
                {mod.option_groups.flatMap(group =>
                  group.options.map(opt => (
                    <div
                      class={
                        "pill " +
                        (!this.props.mod.enabled_options.includes(opt.name) &&
                          "disabled")
                      }>
                      {opt.name}
                    </div>
                  ))
                )}
              </div>
            }
          />
        ) : (
          []
        )}
      </div>
    ) : (
      []
    );
  }
}

const Row = ({ key, val }) => (
  <div class="row">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);

const Long = ({ key, val }) => (
  <div class="long">
    <div class="label">{key}</div>
    <div class="data">{val}</div>
  </div>
);
