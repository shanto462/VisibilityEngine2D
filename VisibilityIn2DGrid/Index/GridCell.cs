using System.Windows.Shapes;

namespace VisibilityIn2DGrid.Index;

public class GridCell
{
    public int Column { get; private init; }
    public int Row { get; private init; }
    public HashSet<Polygon> Polygon { get; private init; }

    public GridCell(int column, int row, float gridSize)
    {
        Column = column;
        Row = row;
        Polygon = [];
    }
}
